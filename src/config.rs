use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginConfig {
    /// Use React Native attribute names (camelCase) instead of web attributes (kebab-case)
    #[serde(default)]
    pub native: bool,

    /// List of component names to ignore during annotation
    #[serde(default, rename = "ignored-components")]
    pub ignored_components: Vec<String>,

    /// Custom component attribute name (overrides default and native setting)
    #[serde(default, rename = "component-attr")]
    pub component_attr: Option<String>,

    /// Custom element attribute name (overrides default and native setting)
    #[serde(default, rename = "element-attr")]
    pub element_attr: Option<String>,

    /// Custom source file attribute name (overrides default and native setting)
    #[serde(default, rename = "source-file-attr")]
    pub source_file_attr: Option<String>,

    /// Custom source path attribute name (overrides default and native setting)
    #[serde(default, rename = "source-path-attr")]
    pub source_path_attr: Option<String>,

    /// Enable rewriting emotion styled components to inject data attributes
    #[serde(default, rename = "rewrite-emotion-styled")]
    pub experimental_rewrite_emotion_styled: bool,
}

impl PluginConfig {
    pub fn component_attr_name(&self) -> &str {
        if let Some(ref custom) = self.component_attr {
            custom
        } else if self.native {
            "dataComponent"
        } else {
            "data-component"
        }
    }

    pub fn element_attr_name(&self) -> &str {
        if let Some(ref custom) = self.element_attr {
            custom
        } else if self.native {
            "dataElement"
        } else {
            "data-element"
        }
    }

    pub fn source_file_attr_name(&self) -> &str {
        if let Some(ref custom) = self.source_file_attr {
            custom
        } else if self.native {
            "dataSourceFile"
        } else {
            "data-source-file"
        }
    }

    pub fn source_path_attr_name(&self) -> &str {
        if let Some(ref custom) = self.source_path_attr {
            custom
        } else if self.native {
            "dataSourcePath"
        } else {
            "data-source-path"
        }
    }
}
