use anyhow::{Context, Result};
use crate::project::Project;
use std::path::Path;
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
    // Try cargo-udeps first (more accurate)
    let udeps_output = Command::new("cargo")
        .args(&["udeps", "--output", "json"])
        .current_dir(&project.path)
        .output();

    if let Ok(output) = udeps_output {
        if output.status.success() {
            // Parse cargo-udeps JSON output
            return parse_udeps_output(&String::from_utf8_lossy(&output.stdout));
        }
    }

    // Fallback to cargo-machete
    let machete_output = Command::new("cargo")
        .arg("machete")
        .current_dir(&project.path)
        .output();

    if let Ok(output) = machete_output {
        if output.status.success() || output.status.code() == Some(1) {
            // cargo-machete exits with code 1 if unused deps found
            return parse_machete_output(&String::from_utf8_lossy(&output.stdout));
        }
    }

    // If neither tool is available, try a simple approach
    // Check if cargo-udeps or cargo-machete are installed
    Ok(vec![])
}

/// Parse cargo-udeps JSON output
fn parse_udeps_output(output: &str) -> Result<Vec<UnusedDependency>> {
    // cargo-udeps JSON format is complex, for now return empty
    // TODO: Implement proper JSON parsing
    Ok(vec![])
}

/// Parse cargo-machete output
fn parse_machete_output(output: &str) -> Result<Vec<UnusedDependency>> {
    let mut unused = Vec::new();
    
    for line in output.lines() {
        // cargo-machete output format: "unused dependency: `dependency_name`"
        if line.contains("unused dependency:") {
            if let Some(start) = line.find('`') {
                if let Some(end) = line[start + 1..].find('`') {
                    let dep_name = &line[start + 1..start + 1 + end];
                    unused.push(UnusedDependency {
                        name: dep_name.to_string(),
                        location: "[dependencies]".to_string(), // machete doesn't specify location
                    });
                }
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
) -> Result<DependencyCleanResult> {
    let unused_deps = check_unused_dependencies(project)
        .with_context(|| format!("Failed to check unused dependencies in {:?}", project.path))?;

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
        let output = "No unused dependencies found.\n";
        let deps = parse_machete_output(output).unwrap();
        assert_eq!(deps.len(), 0);
    }
}

