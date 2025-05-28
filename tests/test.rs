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
use swc_plugin_component_annotate::{config::PluginConfig, ReactComponentAnnotateVisitor};

fn tr_with_config_and_filename(config: PluginConfig, filename: FileName) -> impl Pass {
    let unresolved_mark = Mark::new();
    let top_level_mark = Mark::new();

    (
        resolver(unresolved_mark, top_level_mark, false),
        visit_mut_pass(ReactComponentAnnotateVisitor::new(config, &filename)),
    )
}

#[test]
fn test_extract_filename() {
    use swc_plugin_component_annotate::extract_filename_for_test;

    // Test regular files
    assert_eq!(
        extract_filename_for_test(&FileName::Custom("components/Button.tsx".to_string())),
        Some("Button.tsx".to_string())
    );

    // Test index files - should include parent directory
    assert_eq!(
        extract_filename_for_test(&FileName::Custom("components/Button/index.tsx".to_string())),
        Some("Button/index.tsx".to_string())
    );

    assert_eq!(
        extract_filename_for_test(&FileName::Custom("src/pages/Home/index.jsx".to_string())),
        Some("Home/index.jsx".to_string())
    );

    // Test index files without parent directory
    assert_eq!(
        extract_filename_for_test(&FileName::Custom("index.tsx".to_string())),
        Some("index.tsx".to_string())
    );

    // Test Windows paths
    assert_eq!(
        extract_filename_for_test(&FileName::Custom(
            "components\\Button\\index.tsx".to_string()
        )),
        Some("Button/index.tsx".to_string())
    );

    // Test other index file extensions
    assert_eq!(
        extract_filename_for_test(&FileName::Custom("components/utils/index.ts".to_string())),
        Some("utils/index.ts".to_string())
    );

    assert_eq!(
        extract_filename_for_test(&FileName::Custom("components/helpers/index.js".to_string())),
        Some("helpers/index.js".to_string())
    );
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
    let is_index_test = dir.file_name().unwrap().to_str().unwrap() == "react_index_file";

    let config = if is_sentry_test || is_index_test {
        let mut config = PluginConfig::default();
        config.component_attr = Some("data-sentry-component".to_string());
        config.element_attr = Some("data-sentry-element".to_string());
        config.source_file_attr = Some("data-sentry-source-file".to_string());
        config
    } else {
        PluginConfig::default()
    };

    // Use custom filename for index test
    let filename = if is_index_test {
        FileName::Custom("react_index_file/index.jsx".to_string())
    } else {
        FileName::Custom("test.jsx".to_string())
    };

    test_fixture(
        Syntax::Es(EsSyntax {
            jsx,
            ..Default::default()
        }),
        &|_| tr_with_config_and_filename(config.clone(), filename.clone()),
        &input,
        &output,
        Default::default(),
    );
}
