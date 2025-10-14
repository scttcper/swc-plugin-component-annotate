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
fn test_extract_absolute_path() {
    use swc_plugin_component_annotate::path_utils::extract_absolute_path;

    // Test custom filename (absolute path)
    assert_eq!(
        extract_absolute_path(&FileName::Custom(
            "/Users/jonasbadalic/code/swc-plugin-component-annotate/src/Component.jsx".to_string()
        )),
        Some(
            "/Users/jonasbadalic/code/swc-plugin-component-annotate/src/Component.jsx".to_string()
        )
    );

    // Test relative path
    assert_eq!(
        extract_absolute_path(&FileName::Custom("src/components/Button.tsx".to_string())),
        Some("src/components/Button.tsx".to_string())
    );

    // Test Windows path
    assert_eq!(
        extract_absolute_path(&FileName::Custom(
            "C:\\Users\\Name\\project\\src\\Component.jsx".to_string()
        )),
        Some("C:\\Users\\Name\\project\\src\\Component.jsx".to_string())
    );
}

#[test]
fn test_extract_filename() {
    use swc_plugin_component_annotate::path_utils::extract_filename;

    // Test regular files
    assert_eq!(
        extract_filename(&FileName::Custom("components/Button.tsx".to_string())),
        Some("Button.tsx".to_string())
    );

    // Test index files - should include parent directory
    assert_eq!(
        extract_filename(&FileName::Custom("components/Button/index.tsx".to_string())),
        Some("Button/index.tsx".to_string())
    );

    assert_eq!(
        extract_filename(&FileName::Custom("src/pages/Home/index.jsx".to_string())),
        Some("Home/index.jsx".to_string())
    );

    // Test index files without parent directory
    assert_eq!(
        extract_filename(&FileName::Custom("index.tsx".to_string())),
        Some("index.tsx".to_string())
    );

    // Test Windows paths
    assert_eq!(
        extract_filename(&FileName::Custom(
            "components\\Button\\index.tsx".to_string()
        )),
        Some("Button/index.tsx".to_string())
    );

    // Test other index file extensions
    assert_eq!(
        extract_filename(&FileName::Custom("components/utils/index.ts".to_string())),
        Some("utils/index.ts".to_string())
    );

    assert_eq!(
        extract_filename(&FileName::Custom("components/helpers/index.js".to_string())),
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
    let is_ignored_components_test =
        dir.file_name().unwrap().to_str().unwrap() == "react_ignored_components";
    let is_source_path_test = dir.file_name().unwrap().to_str().unwrap() == "react_source_path";
    let is_inline_styled_test =
        dir.file_name().unwrap().to_str().unwrap() == "react_inline_styled_component";

    let config = if is_sentry_test || is_index_test {
        PluginConfig {
            component_attr: Some("data-sentry-component".to_string()),
            element_attr: Some("data-sentry-element".to_string()),
            source_file_attr: Some("data-sentry-source-file".to_string()),
            ..Default::default()
        }
    } else if is_ignored_components_test {
        PluginConfig {
            ignored_components: vec![
                "IgnoredComponent".to_string(),
                "AnotherIgnoredComponent".to_string(),
                "IgnoredClassComponent".to_string(),
            ],
            ..Default::default()
        }
    } else if is_source_path_test {
        PluginConfig {
            source_path_attr: Some("data-source-path".to_string()),
            ..Default::default()
        }
    } else if is_inline_styled_test {
        PluginConfig {
            experimental_rewrite_emotion_styled: true,
            ..Default::default()
        }
    } else {
        PluginConfig::default()
    };

    // Use custom filename for index test
    let filename = if is_index_test {
        FileName::Custom("react_index_file/index.jsx".to_string())
    } else if is_source_path_test {
        FileName::Custom(
            "/mock/absolute/path/tests/fixture/react_source_path/input.jsx".to_string(),
        )
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

#[test]
fn test_ignored_components_config() {
    use swc_plugin_component_annotate::config::PluginConfig;

    // Test default config has no ignored components
    let default_config = PluginConfig::default();
    assert!(default_config.ignored_components.is_empty());

    // Test JSON parsing with ignored components
    let json_config = r#"{
        "ignored-components": ["TestComponent", "AnotherComponent"],
        "native": false
    }"#;

    let parsed_config: PluginConfig = serde_json::from_str(json_config).unwrap();
    assert_eq!(parsed_config.ignored_components.len(), 2);
    assert!(parsed_config
        .ignored_components
        .contains(&"TestComponent".to_string()));
    assert!(parsed_config
        .ignored_components
        .contains(&"AnotherComponent".to_string()));
    assert!(!parsed_config.native);
}

#[test]
fn test_ignored_components_functionality() {
    use swc_core::common::FileName;
    use swc_plugin_component_annotate::{config::PluginConfig, ReactComponentAnnotateVisitor};

    let config = PluginConfig {
        ignored_components: vec!["IgnoredComponent".to_string()],
        ..Default::default()
    };

    let filename = FileName::Custom("test.jsx".to_string());
    let visitor = ReactComponentAnnotateVisitor::new(config, &filename);

    // Test that the visitor correctly identifies ignored components
    assert!(visitor.should_ignore_component("IgnoredComponent"));
    assert!(!visitor.should_ignore_component("RegularComponent"));
}

#[test]
fn test_plugin_config_parsing() {
    use swc_plugin_component_annotate::config::PluginConfig;

    // Test that the plugin correctly parses JSON configuration with all options
    let config_json = r#"{
        "ignored-components": ["TestIgnored", "AnotherIgnored"],
        "native": true,
        "component-attr": "customComponent",
        "element-attr": "customElement",
        "source-file-attr": "customSourceFile",
        "source-path-attr": "customSourcePath"
    }"#;

    let parsed_config: PluginConfig = serde_json::from_str(config_json).unwrap();

    // Verify all configuration options are parsed correctly
    assert_eq!(parsed_config.ignored_components.len(), 2);
    assert!(parsed_config
        .ignored_components
        .contains(&"TestIgnored".to_string()));
    assert!(parsed_config
        .ignored_components
        .contains(&"AnotherIgnored".to_string()));
    assert!(parsed_config.native);
    assert_eq!(
        parsed_config.component_attr,
        Some("customComponent".to_string())
    );
    assert_eq!(
        parsed_config.element_attr,
        Some("customElement".to_string())
    );
    assert_eq!(
        parsed_config.source_file_attr,
        Some("customSourceFile".to_string())
    );
    assert_eq!(
        parsed_config.source_path_attr,
        Some("customSourcePath".to_string())
    );

    // Test attribute name methods
    assert_eq!(parsed_config.component_attr_name(), "customComponent");
    assert_eq!(parsed_config.element_attr_name(), "customElement");
    assert_eq!(parsed_config.source_file_attr_name(), "customSourceFile");
    assert_eq!(parsed_config.source_path_attr_name(), "customSourcePath");

    // Test native mode with default attributes
    let native_config_json = r#"{
        "native": true
    }"#;

    let native_config: PluginConfig = serde_json::from_str(native_config_json).unwrap();
    assert_eq!(native_config.component_attr_name(), "dataComponent");
    assert_eq!(native_config.element_attr_name(), "dataElement");
    assert_eq!(native_config.source_file_attr_name(), "dataSourceFile");
    assert_eq!(native_config.source_path_attr_name(), "dataSourcePath");
}
