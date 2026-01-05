use anyhow::{Context, Result};
use crate::project::Project;
use std::process::Command;

#[derive(Debug, Clone, serde::Serialize)]
pub struct UnusedDependency {
    pub name: String,
    pub location: String, // e.g., "[dependencies]", "[dev-dependencies]"
}

#[derive(Debug, serde::Serialize)]
pub struct DependencyCleanResult {
    pub path: String,
    pub success: bool,
    pub unused_deps: Vec<UnusedDependency>,
    pub removed_count: usize,
    pub error: Option<String>,
}

/// Check for unused dependencies in a project
pub fn check_unused_dependencies(project: &Project) -> Result<Vec<UnusedDependency>> {
    // Try cargo-machete first (works on stable, simpler)
    let machete_output = Command::new("cargo")
        .arg("machete")
        .current_dir(&project.path)
        .output();

    match &machete_output {
        Ok(output) => {
            // cargo-machete exits with code 1 if unused deps found, 0 if none found
            // But we should check the output regardless of exit code
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            
            // Check both stdout and stderr
            let parsed_stdout = parse_machete_output(&stdout)?;
            if !parsed_stdout.is_empty() {
                return Ok(parsed_stdout);
            }
            let parsed_stderr = parse_machete_output(&stderr)?;
            if !parsed_stderr.is_empty() {
                return Ok(parsed_stderr);
            }
            // If we got here, machete ran but found no unused deps (exit code 0)
            // This is fine, return empty list
        }
        Err(_) => {
            // cargo-machete not available or failed to run, try cargo-udeps
        }
    }

    // Try cargo-udeps (more accurate but requires nightly)
    let udeps_output = Command::new("cargo")
        .args(&["udeps", "--output", "json"])
        .current_dir(&project.path)
        .output();

    if let Ok(output) = &udeps_output {
        // cargo-udeps may exit with non-zero even when it finds unused deps
        // Check if we got any output
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        if !stdout.is_empty() || !stderr.is_empty() {
            // Try parsing the output
            let parsed = parse_udeps_output(&stdout);
            if !parsed.is_empty() {
                return Ok(parsed);
            }
            // Also check stderr for cargo-udeps output
            let parsed_stderr = parse_udeps_output(&stderr);
            if !parsed_stderr.is_empty() {
                return Ok(parsed_stderr);
            }
        }
    }

    // If neither tool is available or found nothing, return empty
    Ok(vec![])
}

/// Parse cargo-udeps JSON output
fn parse_udeps_output(output: &str) -> Vec<UnusedDependency> {
    // cargo-udeps JSON format is complex, for now return empty
    // TODO: Implement proper JSON parsing
    // The JSON structure is: {"unused_deps": [{"name": "...", "location": "..."}]}
    let mut unused = Vec::new();
    
    // Try to parse as JSON
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(output) {
        if let Some(unused_deps) = json.get("unused_deps").and_then(|v| v.as_array()) {
            for dep in unused_deps {
                if let (Some(name), Some(location)) = (
                    dep.get("name").and_then(|v| v.as_str()),
                    dep.get("location").and_then(|v| v.as_str()),
                ) {
                    unused.push(UnusedDependency {
                        name: name.to_string(),
                        location: location.to_string(),
                    });
                }
            }
        }
    }
    
    unused
}

/// Parse cargo-machete output
fn parse_machete_output(output: &str) -> Result<Vec<UnusedDependency>> {
    let mut unused = Vec::new();
    
    // cargo-machete outputs unused dependencies in various formats:
    // - "unused dependency: `dependency_name`"
    // - Just the dependency name in backticks on its own line
    // - Sometimes with "Found X unused dependencies:" header
    
    for line in output.lines() {
        let line = line.trim();
        
        // Skip empty lines and informational messages
        if line.is_empty() 
            || line.contains("Analyzing dependencies")
            || line.contains("didn't find any unused dependencies")
            || line.contains("Good job!")
            || line.contains("Done!")
            || line.contains("Found") && line.contains("unused dependencies") && !line.contains("`") {
            continue;
        }
        
        // Pattern 1: "unused dependency: `name`"
        if line.contains("unused dependency:") {
            if let Some(start) = line.find('`') {
                if let Some(end) = line[start + 1..].find('`') {
                    let dep_name = &line[start + 1..start + 1 + end];
                    if !dep_name.is_empty() {
                        unused.push(UnusedDependency {
                            name: dep_name.to_string(),
                            location: "[dependencies]".to_string(),
                        });
                    }
                }
            }
        } 
        // Pattern 2: Just a dependency name in backticks on its own line
        else if line.starts_with('`') && line.ends_with('`') && line.len() > 2 {
            let dep_name = &line[1..line.len() - 1];
            if !dep_name.is_empty() && !dep_name.contains(' ') && !dep_name.contains(':') {
                unused.push(UnusedDependency {
                    name: dep_name.to_string(),
                    location: "[dependencies]".to_string(),
                });
            }
        }
        // Pattern 3: Sometimes it's just the name without backticks (less common)
        else if !line.contains(" ") && !line.contains(":") && line.len() > 1 && line.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            // This might be a dependency name, but be conservative
            // Only add if it looks like a valid crate name
            if line.chars().next().map(|c| c.is_alphabetic()).unwrap_or(false) {
                unused.push(UnusedDependency {
                    name: line.to_string(),
                    location: "[dependencies]".to_string(),
                });
            }
        }
    }
    
    Ok(unused)
}

/// Remove unused dependencies from Cargo.toml
pub fn remove_unused_dependencies(
    project: &Project,
    unused_deps: &[UnusedDependency],
    dry_run: bool,
) -> Result<usize> {
    if dry_run || unused_deps.is_empty() {
        return Ok(0);
    }

    let cargo_toml = project.path.join("Cargo.toml");
    if !cargo_toml.exists() {
        return Err(anyhow::anyhow!("Cargo.toml not found"));
    }

    // Use cargo-remove if available (from cargo-edit)
    let mut removed = 0;
    for dep in unused_deps {
        let output = Command::new("cargo")
            .args(&["remove", &dep.name])
            .current_dir(&project.path)
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                removed += 1;
            }
        } else {
            // Fallback: manually edit Cargo.toml
            // This is more complex and error-prone, so we skip it for now
            // TODO: Implement manual Cargo.toml editing
        }
    }

    Ok(removed)
}

/// Clean unused dependencies for a project
pub fn clean_dependencies(
    project: &Project,
    dry_run: bool,
    remove: bool,
    verbose: bool,
) -> Result<DependencyCleanResult> {
    // Check if tools are available first
    let machete_available = Command::new("cargo")
        .arg("machete")
        .arg("--version")
        .output()
        .is_ok();
    
    let udeps_available = Command::new("cargo")
        .args(&["udeps", "--version"])
        .output()
        .is_ok();
    
    if !machete_available && !udeps_available {
        return Err(anyhow::anyhow!(
            "No dependency checking tools found. Install cargo-machete (works on stable): cargo install cargo-machete\nOr install cargo-udeps (requires nightly): cargo install cargo-udeps"
        ));
    }

    let unused_deps = check_unused_dependencies(project)
        .with_context(|| format!("Failed to check unused dependencies in {:?}", project.path))?;

    // If verbose and no unused deps found, show which tool was used
    if verbose && unused_deps.is_empty() {
        let tool_used = if machete_available { "cargo-machete" } else { "cargo-udeps" };
        // This will be shown in the main function
    }

    let removed_count = if remove && !unused_deps.is_empty() {
        remove_unused_dependencies(project, &unused_deps, dry_run)
            .unwrap_or(0)
    } else {
        0
    };

    Ok(DependencyCleanResult {
        path: project.path.to_string_lossy().to_string(),
        success: true,
        unused_deps,
        removed_count,
        error: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_machete_output() {
        let output = "unused dependency: `some-crate`\nunused dependency: `another-crate`\n";
        let deps = parse_machete_output(output).unwrap();
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].name, "some-crate");
        assert_eq!(deps[1].name, "another-crate");
    }

    #[test]
    fn test_parse_machete_output_empty() {
        let output = "Analyzing dependencies of crates in this directory...\ncargo-machete didn't find any unused dependencies in this directory. Good job!\nDone!\n";
        let deps = parse_machete_output(output).unwrap();
        assert_eq!(deps.len(), 0);
    }

    #[test]
    fn test_parse_machete_output_real_format() {
        // Real cargo-machete output when it finds unused deps
        let output = "Analyzing dependencies of crates in this directory...\nunused dependency: `some-crate`\nunused dependency: `another-crate`\nDone!\n";
        let deps = parse_machete_output(output).unwrap();
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].name, "some-crate");
        assert_eq!(deps[1].name, "another-crate");
    }

    #[test]
    fn test_parse_machete_output_backticks_only() {
        // Sometimes cargo-machete outputs just the name in backticks
        let output = "`unused-crate`\n`another-one`\n";
        let deps = parse_machete_output(output).unwrap();
        assert_eq!(deps.len(), 2);
    }
}

