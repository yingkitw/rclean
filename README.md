# rclean

A cargo subcommand to recursively clean Cargo projects with workspace support. Available in both **bash script** and **Rust implementation**.

## 🚀 Quick Start

### As a Cargo Plugin (Recommended)

```bash
# Install from local source
cargo install --path .

# Or install from crates.io (when published)
# cargo install rclean

# Use it as a cargo subcommand
cargo rclean [options] [directory]
```

### Standalone Binary

```bash
# Build the tool
cargo build --release

# Use it directly
./target/release/cargo-rclean [options] [directory]
```

### Bash Script

```bash
# Make executable and run
chmod +x cargo-clean-recursive.sh
./cargo-clean-recursive.sh [directory]
```

## 📊 Comparison

| Feature | Bash Script | Rust Implementation |
|---------|-------------|---------------------|
| **Cross-platform** | Partial (OS-specific code) | ✅ Full |
| **Workspace detection** | Fragile (grep-based) | ✅ Robust (cargo-metadata API) |
| **Parallel processing** | ❌ Sequential | ✅ Parallel (configurable) |
| **Dry-run mode** | ❌ No | ✅ Yes (`--dry-run`) |
| **Exclude patterns** | ❌ No | ✅ Yes (`--exclude`) |
| **Progress indication** | ❌ No | ✅ Yes (progress bar) |
| **JSON output** | ❌ No | ✅ Yes (`--json`) |
| **Error handling** | Basic | ✅ Strong (Result types) |
| **Performance** | Sequential | ✅ Parallel (much faster) |
| **Maintainability** | Medium | ✅ High (type-safe) |
| **Distribution** | Script file | ✅ Single binary |

## 🎯 Usage

### As a Cargo Plugin

```bash
# Clean all projects in current directory
cargo rclean

# Clean projects in specific directory
cargo rclean /path/to/projects

# Dry run (preview what would be cleaned)
cargo rclean --dry-run

# Exclude certain patterns
cargo rclean --exclude "**/target" --exclude "**/node_modules"

# Parallel processing with custom job count
cargo rclean -j 8

# Verbose output
cargo rclean --verbose

# JSON output for scripting
cargo rclean --json
```

### As a Standalone Binary

```bash
# Same commands, but use cargo-rclean instead of cargo rclean
cargo-rclean
cargo-rclean /path/to/projects
cargo-rclean --dry-run
# ... etc
```

### Advanced Options

```bash
cargo-cleaner --help
```

**Options:**
- `-j, --jobs <N>`: Number of parallel jobs (default: CPU count)
- `-e, --exclude <PATTERN>`: Exclude glob patterns (can be specified multiple times)
- `--dry-run`: Preview mode (doesn't actually clean)
- `-v, --verbose`: Verbose output
- `--json`: Output results as JSON

## 📝 Bash Script Features

The bash script provides basic recursive cleaning functionality:

```bash
# Clean all Cargo projects in current directory
./cargo-clean-recursive.sh

# Clean all Cargo projects in a specific directory
./cargo-clean-recursive.sh /path/to/projects

# Show help
./cargo-clean-recursive.sh --help
```

### Features
- ✅ Recursive cleaning
- ✅ Basic workspace support
- ✅ Size calculation before/after
- ✅ Color-coded output
- ✅ Error recovery

## 🔧 Requirements

### Rust Implementation
- Rust toolchain (for building)
- Cargo (Rust package manager)

### Bash Script
- Bash shell
- Cargo (Rust package manager)
- Standard Unix utilities (find, grep, etc.)

## 📦 Installation

### As a Cargo Plugin

```bash
# Clone or download this repository
cd cargo-cleaner

# Install as a cargo plugin
cargo install --path .

# Now you can use it as:
cargo rclean
```

**Note:** After installation, make sure `~/.cargo/bin` is in your PATH. The `cargo install` command will show you the installation path.

### As a Standalone Binary

```bash
# Clone or download this repository
cd cargo-cleaner

# Build
cargo build --release

# Use directly
./target/release/cargo-rclean
```

### Bash Script

```bash
# Make executable
chmod +x cargo-clean-recursive.sh

# Use directly
./cargo-clean-recursive.sh
```

## 🎨 Example Output

### Rust Implementation

```
[INFO] Starting cargo clean from: "/path/to/projects"
[INFO] Searching for Cargo projects...
[INFO] Found 5 project(s)

Cleaning: project1  [████████████████████] 5/5 Done!

[INFO] === SUMMARY ===
[SUCCESS] Successfully cleaned: 5 project(s)
[SUCCESS] Total storage freed: 1.23 GB
[SUCCESS] All done!
```

### JSON Output

```json
{
  "total_projects": 5,
  "cleaned": 5,
  "failed": 0,
  "total_freed_bytes": 1321205760,
  "results": [
    {
      "path": "/path/to/project1",
      "success": true,
      "freed_bytes": 524288000,
      "error": null
    }
  ]
}
```

## 🏗️ Architecture

The Rust implementation uses:
- **cargo-metadata**: For proper workspace detection using Cargo's own APIs
- **rayon**: For parallel processing
- **indicatif**: For progress bars
- **clap**: For command-line argument parsing
- **walkdir**: For efficient directory traversal

## 🐛 Error Handling

Both implementations handle:
- ✅ Missing workspace dependencies
- ✅ Malformed Cargo.toml files
- ✅ Failed cargo clean commands (with fallback)
- ✅ Permission errors
- ✅ Missing directories

The Rust implementation provides more detailed error messages and better error recovery.

## 📚 See Also

- [IMPROVEMENTS.md](./IMPROVEMENTS.md) - Detailed analysis and improvement recommendations

## 🤝 Contributing

Improvements and contributions are welcome! The Rust implementation is the recommended path forward for new features.
