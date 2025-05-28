use std::path::PathBuf;
use swc_core::{
    common::{FileName, Mark},
    ecma::{
        ast::Pass,
        parser::{EsSyntax, Syntax},
        transforms::{
            base::resolver,
            testing::{test, test_fixture},
        },
        visit::visit_mut_pass,
    },
};
use swc_plugin_component_annotate::{ReactComponentAnnotateVisitor, config::PluginConfig};

fn tr_with_config(config: PluginConfig) -> impl Pass {
    let unresolved_mark = Mark::new();
    let top_level_mark = Mark::new();
    let filename = FileName::Custom("test.jsx".to_string());

    (
        resolver(unresolved_mark, top_level_mark, false),
        visit_mut_pass(ReactComponentAnnotateVisitor::new(config, &filename)),
    )
}

#[testing::fixture("tests/fixture/react_*/input.jsx")]
fn test(input: PathBuf) {
    let dir = input.parent().unwrap().to_path_buf();
    let jsx = input.extension().unwrap() == "jsx";
    let output = if jsx {
        dir.join("output.jsx")
    } else {
        dir.join("output.js")
    };

    // Check if this is the sentry attrs test
    let is_sentry_test = dir.file_name().unwrap().to_str().unwrap() == "react_sentry_attrs";
    
    let config = if is_sentry_test {
        let mut config = PluginConfig::default();
        config.component_attr = Some("data-sentry-component".to_string());
        config.element_attr = Some("data-sentry-element".to_string());
        config.source_file_attr = Some("data-sentry-source-file".to_string());
        config
    } else {
        PluginConfig::default()
    };
    
    test_fixture(
        Syntax::Es(EsSyntax {
            jsx,
            ..Default::default()
        }),
        &|_| tr_with_config(config.clone()),
        &input,
        &output,
        Default::default(),
    );
}
