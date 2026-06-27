use std::time::{Duration, Instant};

use swc_core::{
    common::{FileName, Mark, SourceMap, GLOBALS},
    ecma::{
        ast::Pass,
        ast::Program,
        parser::{EsSyntax, Parser, StringInput, Syntax},
        transforms::base::resolver,
        visit::visit_mut_pass,
    },
};
use swc_plugin_component_annotate::{config::PluginConfig, ReactComponentAnnotateVisitor};

const WARMUP_ITERS: usize = 3;
const MEASURE_ITERS: usize = 10;

struct BenchCase {
    name: &'static str,
    files: Vec<String>,
    config: PluginConfig,
}

#[derive(Clone, Copy, Default)]
struct BenchStats {
    total: Duration,
    parse: Duration,
    transform: Duration,
}

fn main() {
    let cases = vec![
        BenchCase {
            name: "flat",
            files: generate_files(200, 24, FileShape::Flat),
            config: PluginConfig::default(),
        },
        BenchCase {
            name: "conditional",
            files: generate_files(200, 24, FileShape::Conditional),
            config: PluginConfig::default(),
        },
        BenchCase {
            name: "styled",
            files: generate_files(200, 18, FileShape::Styled),
            config: PluginConfig {
                experimental_rewrite_emotion_styled: true,
                ..Default::default()
            },
        },
        BenchCase {
            name: "preannotated",
            files: generate_files(200, 20, FileShape::Preannotated),
            config: PluginConfig {
                source_path_attr: Some("data-source-path".into()),
                ..Default::default()
            },
        },
    ];

    for case in cases {
        for _ in 0..WARMUP_ITERS {
            run_case(&case);
        }

        let mut total = BenchStats::default();
        let mut best = Duration::MAX;
        for _ in 0..MEASURE_ITERS {
            let stats = run_case(&case);
            total += stats;
            best = best.min(stats.total);
        }

        let files = case.files.len();
        let avg = total / MEASURE_ITERS as u32;
        println!(
            "{:<12} files={:<4} avg_ms={:<8.3} parse_ms={:<8.3} transform_ms={:<8.3} best_ms={:<8.3} avg_us_per_file={:<8.2} best_files_per_sec={:<8.1}",
            case.name,
            files,
            avg.total.as_secs_f64() * 1000.0,
            avg.parse.as_secs_f64() * 1000.0,
            avg.transform.as_secs_f64() * 1000.0,
            best.as_secs_f64() * 1000.0,
            avg.total.as_secs_f64() * 1_000_000.0 / files as f64,
            files as f64 / best.as_secs_f64(),
        );
    }
}

fn run_case(case: &BenchCase) -> BenchStats {
    GLOBALS.set(&Default::default(), || {
        let mut stats = BenchStats::default();
        let total_start = Instant::now();

        for (index, source) in case.files.iter().enumerate() {
            let file_stats = transform_source(source, &case.config, index);
            stats.parse += file_stats.parse;
            stats.transform += file_stats.transform;
        }

        stats.total = total_start.elapsed();
        stats
    })
}

fn transform_source(source: &str, config: &PluginConfig, index: usize) -> BenchStats {
    let mut stats = BenchStats::default();
    let source_map = SourceMap::default();
    let filename = FileName::Custom(format!("bench/File{index}.jsx"));
    let source_file = source_map.new_source_file(filename.clone().into(), source.to_string());
    let mut parser = Parser::new(
        Syntax::Es(EsSyntax {
            jsx: true,
            ..Default::default()
        }),
        StringInput::from(&*source_file),
        None,
    );
    let parse_start = Instant::now();
    let module = parser.parse_module().expect("bench source should parse");
    stats.parse = parse_start.elapsed();
    let mut program = Program::Module(module);

    let unresolved_mark = Mark::new();
    let top_level_mark = Mark::new();
    let mut pass = (
        resolver(unresolved_mark, top_level_mark, false),
        visit_mut_pass(ReactComponentAnnotateVisitor::new(
            config.clone(),
            &filename,
        )),
    );

    let transform_start = Instant::now();
    pass.process(&mut program);
    stats.transform = transform_start.elapsed();
    std::hint::black_box(program);
    stats
}

#[derive(Clone, Copy)]
enum FileShape {
    Flat,
    Conditional,
    Styled,
    Preannotated,
}

fn generate_files(file_count: usize, component_count: usize, shape: FileShape) -> Vec<String> {
    (0..file_count)
        .map(|file_index| generate_file(file_index, component_count, shape))
        .collect()
}

fn generate_file(file_index: usize, component_count: usize, shape: FileShape) -> String {
    let mut source = String::with_capacity(component_count * 900);
    source.push_str("import React, { Fragment } from 'react';\n");

    if matches!(shape, FileShape::Styled) {
        source.push_str("import styled from '@emotion/styled';\n");
        source.push_str("const BaseButton = (props) => <button {...props} />;\n");
        source.push_str("const StyledButton = styled(BaseButton);\n");
    }

    source.push_str("const Provider = ({children}) => <main>{children}</main>;\n");
    source.push_str("const UserProvider = ({children}) => <section>{children}</section>;\n");

    for component_index in 0..component_count {
        let name = format!("Component{file_index}_{component_index}");
        match shape {
            FileShape::Flat => push_flat_component(&mut source, &name),
            FileShape::Conditional => push_conditional_component(&mut source, &name),
            FileShape::Styled => push_styled_component(&mut source, &name),
            FileShape::Preannotated => push_preannotated_component(&mut source, &name),
        }
    }

    source
}

fn push_flat_component(source: &mut String, name: &str) {
    source.push_str("function ");
    source.push_str(name);
    source.push_str("({enabled, children}) {\n");
    source.push_str("  const DynamicProvider = enabled ? UserProvider : Fragment;\n");
    source.push_str("  return <div><Header /><DynamicProvider>{children}</DynamicProvider><Content.Card><Content.Card.Body><button>Save</button></Content.Card.Body></Content.Card></div>;\n");
    source.push_str("}\n");
}

fn push_conditional_component(source: &mut String, name: &str) {
    source.push_str("function ");
    source.push_str(name);
    source.push_str("({enabled, items, children}) {\n");
    source.push_str("  const DynamicProvider = enabled ? UserProvider : Fragment;\n");
    source.push_str("  return <><DynamicProvider>{enabled && <Panel>{items.map((item) => <Row key={item.id}>{item.label ? <strong>{item.label}</strong> : <span>empty</span>}</Row>)}</Panel>}</DynamicProvider>{children}</>;\n");
    source.push_str("}\n");
}

fn push_styled_component(source: &mut String, name: &str) {
    source.push_str("const ");
    source.push_str(name);
    source.push_str(" = ({enabled, children}) => {\n");
    source.push_str("  const DynamicProvider = enabled ? UserProvider : Fragment;\n");
    source.push_str("  return <Stack><StyledButton /><DynamicProvider>{enabled ? <StyledButton>{children}</StyledButton> : <Fragment>{children}</Fragment>}</DynamicProvider></Stack>;\n");
    source.push_str("};\n");
}

fn push_preannotated_component(source: &mut String, name: &str) {
    source.push_str("function ");
    source.push_str(name);
    source.push_str("({enabled, props, children}) {\n");
    source.push_str("  const DynamicProvider = enabled ? UserProvider : Fragment;\n");
    source.push_str("  return <article className=\"frame\" role=\"region\" data-component=\"");
    source.push_str(name);
    source.push_str("\" data-element=\"article\" data-source-file=\"File.jsx\" data-source-path=\"/bench/File.jsx\" {...props}><DynamicProvider>{enabled ? <Card className=\"card\" data-component=\"");
    source.push_str(name);
    source.push_str("\" data-element=\"Card\" data-source-file=\"File.jsx\" data-source-path=\"/bench/File.jsx\">{children}</Card> : <Fragment>{children}</Fragment>}</DynamicProvider></article>;\n");
    source.push_str("}\n");
}

impl std::ops::AddAssign for BenchStats {
    fn add_assign(&mut self, rhs: Self) {
        self.total += rhs.total;
        self.parse += rhs.parse;
        self.transform += rhs.transform;
    }
}

impl std::ops::Div<u32> for BenchStats {
    type Output = Self;

    fn div(self, rhs: u32) -> Self::Output {
        Self {
            total: self.total / rhs,
            parse: self.parse / rhs,
            transform: self.transform / rhs,
        }
    }
}
