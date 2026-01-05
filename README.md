# deepclean 🧹

> A fast, parallel tool to clean Rust projects and remove unused dependencies

## Why deepclean?

Rust projects can accumulate gigabytes of build artifacts in `target/` directories. When you have multiple projects or workspaces, manually cleaning them is tedious and time-consuming. Plus, unused dependencies bloat your `Cargo.toml` and slow down builds.

**deepclean solves this by:**
- 🚀 **Cleaning multiple projects in parallel** - Save time with concurrent cleaning
- 🎯 **Smart workspace detection** - Automatically finds all Cargo projects
- 🧹 **Removes unused dependencies** - Keep your `Cargo.toml` clean
- ⚡ **Fast and efficient** - Built in Rust for maximum performance

## Quick Start

### Installation

```bash
# Clone the repository
git clone https://github.com/yingkitw/deepclean.git
cd deepclean

# Install as a cargo plugin
cargo install --path .

# Now use it!
cargo deepclean
```

**Note:** Make sure `~/.cargo/bin` is in your PATH.

### Basic Usage

```bash
# Clean current directory and all subdirectories
cargo deepclean

# Clean a specific directory
cargo deepclean /path/to/projects

# Preview what would be cleaned (dry run)
cargo deepclean --dry-run

# Only clean projects above 100MB
cargo deepclean --min-size 100MB

# Check for unused dependencies
cargo deepclean --clean-deps

# Remove unused dependencies (automatically checks first)
cargo deepclean --remove-deps
```

## Features

- ✅ **Parallel processing** - Clean multiple projects simultaneously
- ✅ **Smart detection** - Uses cargo-metadata for accurate workspace detection
- ✅ **Dependency cleaning** - Find and remove unused dependencies (built-in detection)
- ✅ **Size filtering** - Only clean projects above a certain size
- ✅ **Progress bars** - See what's being cleaned in real-time
- ✅ **Dry-run mode** - Preview changes before applying them
- ✅ **Exclude patterns** - Skip specific directories
- ✅ **JSON output** - Machine-readable output for automation

## Options

| Option | Description |
|--------|-------------|
| `-j, --jobs <N>` | Number of parallel jobs (default: CPU count) |
| `-e, --exclude <PATTERN>` | Exclude directories matching pattern (can use multiple times) |
| `--dry-run` | Preview mode (doesn't actually clean) |
| `--min-size <SIZE>` | Only clean projects above this size (e.g., "100MB", "1GB") |
| `--clean-deps` | Check for unused dependencies |
| `--remove-deps` | Remove unused dependencies (requires `cargo-remove`) |
| `-v, --verbose` | Verbose output |
| `--json` | Output results as JSON |

## Requirements

- Rust toolchain
- Cargo

### Optional: Dependency Removal

To remove unused dependencies, install `cargo-edit`:

```bash
cargo install cargo-edit
```

**Note:** Dependency detection is built-in and doesn't require external tools! The tool parses `Cargo.toml` and searches your source code to find unused dependencies.

## Examples

### Clean Everything

```bash
cargo deepclean
```

### Clean Only Large Projects

```bash
cargo deepclean --min-size 500MB
```

### Find Unused Dependencies

```bash
cargo deepclean --clean-deps
```

### Remove Unused Dependencies

```bash
cargo deepclean --remove-deps
```

### Exclude Specific Directories

```bash
cargo deepclean --exclude "**/target/debug" --exclude "**/node_modules"
```

### Parallel Cleaning with Custom Jobs

```bash
cargo deepclean -j 8
```

## How It Works

1. **Discovery**: Recursively finds all Cargo projects using `cargo-metadata`
2. **Filtering**: Optionally filters by size or exclude patterns
3. **Cleaning**: Removes `target/` directories in parallel
4. **Dependency Analysis**: Parses `Cargo.toml` and searches source code for unused dependencies
5. **Removal**: Uses `cargo-remove` to clean up unused dependencies

## Performance

deepclean is built in Rust for maximum performance:
- Parallel execution across all CPU cores
- Efficient directory traversal
- Minimal memory footprint

## Contributing

Contributions are welcome! Feel free to open issues or submit pull requests.

## License

Apache-2.0

## Repository

https://github.com/yingkitw/deepclean
