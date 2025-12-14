//! Find files by name pattern tool
//!
//! This tool finds files matching a glob pattern or regex in the file system.

use crate::api::agent::{Tool, ToolSchema, ToolSchemaBuilder};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Parameters for the find files tool
#[derive(Debug, Deserialize)]
pub struct FindFilesParams {
    /// The pattern to match file names against (supports glob patterns like *.rs)
    pub pattern: String,
    /// The directory path to search in (default: current directory)
    pub path: Option<String>,
    /// Whether to use regex for pattern matching (default: false for glob)
    pub regex: Option<bool>,
    /// Whether to search recursively (default: true)
    pub recursive: Option<bool>,
    /// Maximum number of results to return (default: 100)
    pub max_results: Option<usize>,
    /// File extensions to include (e.g., ["rs", "py"])
    pub extensions: Option<Vec<String>>,
}

/// A single found file
#[derive(Debug, Serialize)]
pub struct FoundFile {
    /// Path to the file
    pub path: String,
    /// File name only
    pub name: String,
    /// File size in bytes
    pub size: u64,
    /// Type: "file", "directory", or "symlink"
    pub file_type: String,
}

/// Result from find files
#[derive(Debug, Serialize)]
pub struct FindFilesResult {
    /// Files that matched the pattern
    pub files: Vec<FoundFile>,
    /// The pattern that was searched for
    pub pattern: String,
    /// The path that was searched
    pub search_path: String,
    /// Total number of matches found
    pub total_matches: usize,
    /// Whether the result limit was reached (more results exist)
    pub limit_reached: bool,
    /// Whether the search was successful
    pub success: bool,
}

/// Default maximum number of results to return
const DEFAULT_MAX_RESULTS: usize = 100;

/// Find files tool
///
/// Finds files by name pattern with support for:
/// - Glob patterns (e.g., "*.rs", "src/**/*.rs")
/// - Regular expressions
/// - Recursive directory traversal
/// - File extension filtering
/// - Result limiting
pub struct FindFilesTool;

impl FindFilesTool {
    /// Create a new FindFilesTool instance
    pub fn new() -> Self {
        Self
    }

    fn matches_pattern(&self, name: &str, pattern: &str, use_regex: bool) -> Result<bool, String> {
        if use_regex {
            let re =
                regex::Regex::new(pattern).map_err(|e| format!("Invalid regex pattern: {}", e))?;
            Ok(re.is_match(name))
        } else {
            // Use glob matching
            let glob =
                globset::Glob::new(pattern).map_err(|e| format!("Invalid glob pattern: {}", e))?;
            let matcher = glob.compile_matcher();
            Ok(matcher.is_match(name))
        }
    }

    fn find_files_recursive(
        &self,
        path: &Path,
        pattern: &str,
        use_regex: bool,
        extensions: &Option<Vec<String>>,
        results: &mut Vec<FoundFile>,
        total_count: &mut usize,
        max_results: usize,
    ) -> Result<(), String> {
        if *total_count >= max_results {
            return Ok(());
        }

        if path.is_file() {
            let name = path
                .file_name()
                .ok_or("Invalid file name")?
                .to_string_lossy();

            // Check extension filter
            if let Some(exts) = extensions {
                if let Some(ext) = path.extension() {
                    let ext_str = ext.to_string_lossy().to_lowercase();
                    if !exts.iter().any(|e| e.to_lowercase() == ext_str) {
                        return Ok(());
                    }
                } else {
                    return Ok(());
                }
            }

            // Check if name matches pattern
            if self.matches_pattern(&name, pattern, use_regex)? {
                let metadata =
                    fs::metadata(path).map_err(|e| format!("Failed to read metadata: {}", e))?;

                let file_type = if metadata.file_type().is_symlink() {
                    "symlink".to_string()
                } else if metadata.file_type().is_dir() {
                    "directory".to_string()
                } else {
                    "file".to_string()
                };

                if results.len() < max_results {
                    results.push(FoundFile {
                        path: path.to_string_lossy().to_string(),
                        name: name.to_string(),
                        size: metadata.len(),
                        file_type,
                    });
                }
                *total_count += 1;
            }
        } else if path.is_dir() {
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.flatten() {
                    let entry_path = entry.path();
                    // Skip hidden files and common ignore patterns
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();
                    if name_str.starts_with('.')
                        || name_str == "node_modules"
                        || name_str == "target"
                    {
                        continue;
                    }
                    self.find_files_recursive(
                        &entry_path,
                        pattern,
                        use_regex,
                        extensions,
                        results,
                        total_count,
                        max_results,
                    )?;
                }
            }
        }

        Ok(())
    }
}

impl Default for FindFilesTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for FindFilesTool {
    type Params = FindFilesParams;
    type Result = FindFilesResult;

    fn name(&self) -> &str {
        "find_files"
    }

    fn description(&self) -> &str {
        "Find files by name pattern using glob patterns or regex. Results are limited to prevent API errors."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchemaBuilder::new("find_files", "Find files by name pattern")
            .param("pattern", "string")
            .description(
                "pattern",
                "Pattern to match file names (supports glob like *.rs or regex)",
            )
            .required("pattern")
            .param("path", "string")
            .description(
                "path",
                "Directory to search in (default: current directory)",
            )
            .param("regex", "boolean")
            .description(
                "regex",
                "Use regex matching instead of glob (default: false)",
            )
            .param("recursive", "boolean")
            .description(
                "recursive",
                "Search directories recursively (default: true)",
            )
            .param("max_results", "integer")
            .description("max_results", "Maximum files to return (default: 100)")
            .param("extensions", "array")
            .description(
                "extensions",
                "File extensions to include, e.g. [\"rs\", \"py\"]",
            )
            .build()
    }

    async fn execute(&self, params: Self::Params) -> Result<Self::Result, String> {
        let FindFilesParams {
            pattern,
            path,
            regex,
            recursive,
            max_results,
            extensions,
        } = params;

        if pattern.is_empty() {
            return Err("Pattern cannot be empty".to_string());
        }

        let search_path = path.unwrap_or_else(|| ".".to_string());
        let use_regex = regex.unwrap_or(false);
        let recursive = recursive.unwrap_or(true);
        let max_results = max_results.unwrap_or(DEFAULT_MAX_RESULTS);

        let path = Path::new(&search_path);
        if !path.exists() {
            return Err(format!("Path '{}' does not exist", search_path));
        }

        let mut results = Vec::new();
        let mut total_count = 0;

        if recursive {
            self.find_files_recursive(
                path,
                &pattern,
                use_regex,
                &extensions,
                &mut results,
                &mut total_count,
                max_results,
            )?;
        } else {
            // Non-recursive: only search the immediate directory
            if path.is_dir() {
                if let Ok(entries) = fs::read_dir(path) {
                    for entry in entries.flatten() {
                        let entry_path = entry.path();
                        if total_count >= max_results {
                            break;
                        }

                        if entry_path.is_file() {
                            let name = entry_path.file_name().unwrap().to_string_lossy();

                            // Check extension filter
                            if let Some(exts) = &extensions {
                                if let Some(ext) = entry_path.extension() {
                                    let ext_str = ext.to_string_lossy().to_lowercase();
                                    if !exts.iter().any(|e| e.to_lowercase() == ext_str) {
                                        continue;
                                    }
                                } else {
                                    continue;
                                }
                            }

                            // Check if name matches pattern
                            if self.matches_pattern(&name, &pattern, use_regex)? {
                                let metadata = fs::metadata(&entry_path)
                                    .map_err(|e| format!("Failed to read metadata: {}", e))?;

                                results.push(FoundFile {
                                    path: entry_path.to_string_lossy().to_string(),
                                    name: name.to_string(),
                                    size: metadata.len(),
                                    file_type: "file".to_string(),
                                });
                                total_count += 1;
                            }
                        }
                    }
                }
            }
        }

        let limit_reached = total_count > max_results;
        Ok(FindFilesResult {
            files: results,
            pattern,
            search_path,
            total_matches: total_count,
            limit_reached,
            success: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_find_files_glob() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("test.rs"), "content").unwrap();
        fs::write(temp_dir.path().join("test.py"), "content").unwrap();
        fs::write(temp_dir.path().join("other.txt"), "content").unwrap();

        let tool = FindFilesTool::new();
        let result = tool
            .execute(FindFilesParams {
                pattern: "*.rs".to_string(),
                path: Some(temp_dir.path().to_string_lossy().to_string()),
                regex: Some(false),
                recursive: Some(false),
                max_results: None,
                extensions: None,
            })
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.files.len(), 1);
        assert_eq!(result.files[0].name, "test.rs");
        assert_eq!(result.total_matches, 1);
        assert!(!result.limit_reached);
    }

    #[tokio::test]
    async fn test_find_files_with_limit() {
        let temp_dir = TempDir::new().unwrap();
        for i in 0..10 {
            fs::write(temp_dir.path().join(format!("test{}.txt", i)), "content").unwrap();
        }

        let tool = FindFilesTool::new();
        let result = tool
            .execute(FindFilesParams {
                pattern: "*.txt".to_string(),
                path: Some(temp_dir.path().to_string_lossy().to_string()),
                regex: Some(false),
                recursive: Some(false),
                max_results: Some(5),
                extensions: None,
            })
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.files.len(), 5);
        assert_eq!(result.total_matches, 10);
        assert!(result.limit_reached);
    }
}
