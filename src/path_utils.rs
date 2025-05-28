use swc_core::common::FileName;

// Platform-specific path parsing functions
fn parse_unix_path(path: &str) -> Vec<&str> {
    path.split('/').collect()
}

fn parse_windows_path(path: &str) -> Vec<&str> {
    path.split('\\').collect()
}

// Fallback for mixed or unknown paths - checks the string content
fn parse_path_with_detection(path: &str) -> Vec<&str> {
    if path.contains('\\') {
        parse_windows_path(path)
    } else {
        parse_unix_path(path)
    }
}

pub fn extract_filename(filename: &FileName) -> Option<String> {
    match filename {
        FileName::Real(path) => {
            if let Some(file_name) = path.file_name().and_then(|name| name.to_str()) {
                // Check if it's an index file
                if file_name.starts_with("index.") {
                    // Get parent directory name and combine with filename
                    if let Some(parent) = path
                        .parent()
                        .and_then(|p| p.file_name())
                        .and_then(|n| n.to_str())
                    {
                        Some(format!("{}/{}", parent, file_name))
                    } else {
                        Some(file_name.to_string())
                    }
                } else {
                    Some(file_name.to_string())
                }
            } else {
                None
            }
        }
        FileName::Custom(custom) => {
            // Always use detection for Custom filenames since they can come from any platform
            let parts = parse_path_with_detection(custom);

            let file_part = if parts.len() > 1 {
                parts.last().copied().unwrap_or(custom)
            } else {
                custom
            };

            // Check if it's an index file
            if file_part.starts_with("index.") {
                // Extract parent directory from the full path
                let parent = if parts.len() >= 2 {
                    Some(parts[parts.len() - 2])
                } else {
                    None
                };

                if let Some(parent_name) = parent {
                    // Always use forward slash in output for consistency
                    Some(format!("{}/{}", parent_name, file_part))
                } else {
                    Some(file_part.to_string())
                }
            } else {
                Some(file_part.to_string())
            }
        }
        _ => None,
    }
}
