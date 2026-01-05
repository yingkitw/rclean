use anyhow::{Context, Result};
use clap::Parser;
use colored::*;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(name = "cargo-rclean")]
#[command(about = "Recursively clean Cargo projects with workspace support", long_about = None)]
#[command(bin_name = "cargo rclean")]
struct Args {
    /// Directory to start cleaning from
    #[arg(default_value = ".")]
    directory: PathBuf,

    /// Dry run mode (don't actually clean, just show what would be cleaned)
    #[arg(long)]
    dry_run: bool,

    /// Exclude patterns (glob patterns, can be specified multiple times)
    #[arg(short = 'e', long = "exclude")]
    exclude_patterns: Vec<String>,

    /// Number of parallel jobs
    #[arg(short = 'j', long = "jobs", default_value_t = num_cpus::get())]
    jobs: usize,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    /// JSON output
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Clone)]
struct Project {
    path: PathBuf,
    is_workspace: bool,
}

#[derive(Debug, serde::Serialize)]
struct CleanResult {
    path: String,
    success: bool,
    freed_bytes: u64,
    error: Option<String>,
}

#[derive(Debug, serde::Serialize)]
struct Summary {
    total_projects: usize,
    cleaned: usize,
    failed: usize,
    total_freed_bytes: u64,
    results: Vec<CleanResult>,
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;

    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }

    if unit_idx == 0 {
        format!("{} {}", bytes, UNITS[unit_idx])
    } else {
        format!("{:.2} {}", size, UNITS[unit_idx])
    }
}

fn get_directory_size(path: &Path) -> Result<u64> {
    let mut total = 0u64;
    if !path.exists() {
        return Ok(0);
    }

    for entry in WalkDir::new(path) {
        let entry = entry?;
        if entry.file_type().is_file() {
            total += entry.metadata()?.len();
        }
    }
    Ok(total)
}

fn find_cargo_projects(root: &Path, exclude_patterns: &[String]) -> Result<Vec<Project>> {
    let mut projects = Vec::new();
    let mut seen_workspaces = HashSet::new();

    for entry in WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            // Skip hidden directories and common exclusions
            let name = e.file_name().to_string_lossy();
            if name.starts_with('.') && name != "." && name != ".." {
                return false;
            }

            // Check exclude patterns
            for pattern in exclude_patterns {
                if glob::Pattern::new(pattern)
                    .ok()
                    .and_then(|p| {
                        e.path()
                            .strip_prefix(root)
                            .ok()
                            .and_then(|rel| Some(p.matches(&rel.to_string_lossy())))
                    })
                    .unwrap_or(false)
                {
                    return false;
                }
            }
            true
        })
    {
        let entry = entry?;
        if entry.file_name() == "Cargo.toml" {
            let project_dir = entry.path().parent().unwrap().to_path_buf();

            // Check if this is part of a workspace
            let mut is_workspace_member = false;
            let mut current = project_dir.parent();
            while let Some(parent) = current {
                let workspace_toml = parent.join("Cargo.toml");
                if workspace_toml.exists() {
                    // Try to parse as workspace
                    if let Ok(metadata) = cargo_metadata::MetadataCommand::new()
                        .manifest_path(&workspace_toml)
                        .exec()
                    {
                        if metadata.workspace_root == parent {
                            // This is a workspace member
                            let workspace_path: PathBuf = metadata.workspace_root.into();
                            if !seen_workspaces.contains(&workspace_path) {
                                seen_workspaces.insert(workspace_path.clone());
                                projects.push(Project {
                                    path: workspace_path,
                                    is_workspace: true,
                                });
                            }
                            is_workspace_member = true;
                            break;
                        }
                    }
                }
                current = parent.parent();
            }

            // If not a workspace member, add as standalone project
            if !is_workspace_member {
                projects.push(Project {
                    path: project_dir,
                    is_workspace: false,
                });
            }
        }
    }

    // Remove duplicates
    projects.sort_by_key(|p| p.path.clone());
    projects.dedup_by_key(|p| p.path.clone());

    Ok(projects)
}

fn clean_project(project: &Project, dry_run: bool, _verbose: bool) -> Result<CleanResult> {
    let target_dir = project.path.join("target");
    let freed_bytes = if target_dir.exists() {
        get_directory_size(&target_dir).unwrap_or(0)
    } else {
        0
    };

    if dry_run {
        return Ok(CleanResult {
            path: project.path.to_string_lossy().to_string(),
            success: true,
            freed_bytes,
            error: None,
        });
    }

    // Try cargo clean first
    let output = Command::new("cargo")
        .arg("clean")
        .current_dir(&project.path)
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let after_size = if target_dir.exists() {
                get_directory_size(&target_dir).unwrap_or(0)
            } else {
                0
            };
            let actually_freed = freed_bytes.saturating_sub(after_size);

            Ok(CleanResult {
                path: project.path.to_string_lossy().to_string(),
                success: true,
                freed_bytes: actually_freed,
                error: None,
            })
        }
        _ => {
            // Fallback: remove target directory directly
            if target_dir.exists() {
                std::fs::remove_dir_all(&target_dir)
                    .with_context(|| format!("Failed to remove target directory: {:?}", target_dir))?;

                Ok(CleanResult {
                    path: project.path.to_string_lossy().to_string(),
                    success: true,
                    freed_bytes,
                    error: None,
                })
            } else {
                Ok(CleanResult {
                    path: project.path.to_string_lossy().to_string(),
                    success: true,
                    freed_bytes: 0,
                    error: None,
                })
            }
        }
    }
}

fn main() -> Result<()> {
    // Handle being called as a cargo subcommand
    // When invoked as `cargo rclean`, cargo passes "rclean" as the first argument
    let mut args_iter = std::env::args();
    let program_name = args_iter.next();
    
    // Check if we're being called as `cargo rclean` (first arg is "rclean")
    let args = if args_iter.next().as_deref() == Some("rclean") {
        // Skip "rclean" and parse the rest
        Args::parse_from(args_iter)
    } else {
        // Called directly as `cargo-rclean`, reconstruct args
        let mut all_args = vec![program_name.unwrap_or_else(|| "cargo-rclean".to_string())];
        all_args.extend(args_iter);
        Args::parse_from(all_args)
    };
    let root = args.directory.canonicalize()
        .with_context(|| format!("Failed to canonicalize path: {:?}", args.directory))?;

    if !args.json {
        println!("{} {}", "[INFO]".blue().bold(), format!("Starting cargo clean from: {:?}", root));
        println!("{} Searching for Cargo projects...", "[INFO]".blue().bold());
    }

    let projects = find_cargo_projects(&root, &args.exclude_patterns)
        .context("Failed to find Cargo projects")?;

    if projects.is_empty() {
        if !args.json {
            println!("{} No Cargo projects found", "[WARNING]".yellow().bold());
        }
        return Ok(());
    }

    if !args.json {
        println!("{} Found {} project(s)", "[INFO]".blue().bold(), projects.len());
        if args.dry_run {
            println!("{} DRY RUN MODE - no changes will be made", "[INFO]".yellow().bold());
        }
        println!();
    }

    let multi = if !args.json && !args.verbose {
        Some(Arc::new(MultiProgress::new()))
    } else {
        None
    };

    // Create overall progress bar
    let overall_pb = if let Some(ref multi) = multi {
        let pb = multi.add(ProgressBar::new(projects.len() as u64));
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} projects completed")
                .unwrap()
                .progress_chars("#>-"),
        );
        pb.set_message("Starting...");
        Some(pb)
    } else {
        None
    };

    let results: Vec<CleanResult> = projects
        .par_iter()
        .with_min_len(1)
        .map(|project| {
            // Create individual progress bar for this project
            let project_pb = if let Some(ref multi) = multi {
                let pb = multi.add(ProgressBar::new_spinner());
                pb.set_style(
                    ProgressStyle::default_spinner()
                        .template("{spinner:.green} {msg}")
                        .unwrap()
                        .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
                );
                let project_name = project.path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| project.path.to_string_lossy().to_string());
                pb.set_message(format!("Cleaning: {}", project_name));
                pb.enable_steady_tick(std::time::Duration::from_millis(100));
                Some(pb)
            } else {
                None
            };

            if args.verbose && !args.json {
                println!("{} Cleaning: {:?}", "[INFO]".blue().bold(), project.path);
            }

            let result = clean_project(project, args.dry_run, args.verbose);

            // Finish individual progress bar
            if let Some(ref pb) = project_pb {
                let project_name = project.path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| project.path.to_string_lossy().to_string());
                pb.finish_with_message(format!("✓ {}", project_name));
            }

            // Update overall progress
            if let Some(ref overall) = overall_pb {
                overall.inc(1);
            }

            match result {
                Ok(r) => {
                    if args.verbose && !args.json {
                        if r.freed_bytes > 0 {
                            println!(
                                "{} Cleaned: {:?} (freed: {})",
                                "[SUCCESS]".green().bold(),
                                r.path,
                                format_bytes(r.freed_bytes)
                            );
                        } else {
                            println!(
                                "{} Cleaned: {:?} (already clean)",
                                "[SUCCESS]".green().bold(),
                                r.path
                            );
                        }
                    }
                    Ok(r)
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    if !args.json {
                        println!(
                            "{} Failed to clean: {:?} - {}",
                            "[ERROR]".red().bold(),
                            project.path,
                            error_msg
                        );
                    }
                    Ok(CleanResult {
                        path: project.path.to_string_lossy().to_string(),
                        success: false,
                        freed_bytes: 0,
                        error: Some(error_msg),
                    })
                }
            }
        })
        .collect::<Result<Vec<_>>>()?;

    if let Some(ref overall) = overall_pb {
        overall.finish_with_message("All projects completed!");
    }

    let cleaned = results.iter().filter(|r| r.success).count();
    let failed = results.len() - cleaned;
    let total_freed: u64 = results.iter().map(|r| r.freed_bytes).sum();

    let summary = Summary {
        total_projects: projects.len(),
        cleaned,
        failed,
        total_freed_bytes: total_freed,
        results,
    };

    if args.json {
        println!("{}", serde_json::to_string_pretty(&summary)?);
    } else {
        println!();
        println!("{} {}", "[INFO]".blue().bold(), "=== SUMMARY ===");
        println!(
            "{} Successfully cleaned: {} project(s)",
            "[SUCCESS]".green().bold(),
            cleaned
        );

        if total_freed > 0 {
            println!(
                "{} Total storage freed: {}",
                "[SUCCESS]".green().bold(),
                format_bytes(total_freed)
            );
        } else {
            println!("{} No storage was freed", "[INFO]".blue().bold());
        }

        if failed > 0 {
            println!(
                "{} Failed to clean: {} project(s)",
                "[ERROR]".red().bold(),
                failed
            );
        } else {
            println!("{} All done!", "[SUCCESS]".green().bold());
        }
    }

    if failed > 0 {
        std::process::exit(1);
    }

    Ok(())
}

