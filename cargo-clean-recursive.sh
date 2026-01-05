#!/bin/bash

# Simple cargo clean recursive script
# Recursively runs 'cargo clean' in all directories containing Cargo.toml

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

print_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
print_success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }
print_warning() { echo -e "${YELLOW}[WARNING]${NC} $1"; }
print_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Get directory size in bytes
get_size_bytes() {
    local dir="$1"
    if [ -d "$dir" ]; then
        if [[ "$OSTYPE" == "darwin"* ]]; then
            find "$dir" -type f -exec stat -f%z {} \; 2>/dev/null | awk '{sum+=$1} END {print sum}'
        else
            du -sb "$dir" 2>/dev/null | cut -f1
        fi
    else
        echo "0"
    fi
}

# Format bytes to human readable
format_bytes() {
    local bytes="$1"
    [ "$bytes" -eq 0 ] 2>/dev/null && echo "0 B" && return

    if [ "$bytes" -ge 1073741824 ]; then
        echo "$((bytes / 1073741824)) GB"
    elif [ "$bytes" -ge 1048576 ]; then
        echo "$((bytes / 1048576)) MB"
    elif [ "$bytes" -ge 1024 ]; then
        echo "$((bytes / 1024)) KB"
    else
        echo "${bytes} B"
    fi
}

# Clean a single cargo project
clean_project() {
    local project_dir="$1"
    local total_freed_var="$2"

    print_info "Cleaning: $project_dir"

    # Get target size before cleaning
    local target_before=0
    [ -d "$project_dir/target" ] && target_before=$(get_size_bytes "$project_dir/target")
    [ -z "$target_before" ] && target_before=0

    # Change to project directory
    cd "$project_dir" || {
        print_error "Failed to enter: $project_dir"
        return 1
    }

    # Try cargo clean
    if cargo clean 2>/dev/null; then
        # Success - calculate freed space
        local target_after=0
        [ -d "target" ] && target_after=$(get_size_bytes "target")
        [ -z "$target_after" ] && target_after=0

        local freed=$((target_before - target_after))
        eval "$total_freed_var=\$((\$$total_freed_var + freed))"

        if [ "$freed" -gt 0 ]; then
            local freed_human=$(format_bytes "$freed")
            print_success "Cleaned: $project_dir (freed: $freed_human)"
        else
            print_success "Cleaned: $project_dir (already clean)"
        fi
        return 0
    else
        # Try direct target removal
        if [ -d "target" ]; then
            print_warning "Cargo clean failed, removing target directory..."
            if rm -rf target; then
                eval "$total_freed_var=\$((\$$total_freed_var + target_before))"
                local freed_human=$(format_bytes "$target_before")
                print_success "Removed target: $project_dir (freed: $freed_human)"
                return 0
            fi
        fi

        print_error "Failed to clean: $project_dir"
        return 1
    fi
}

# Main function
main() {
    local start_dir="${1:-.}"
    start_dir=$(realpath "$start_dir")

    local cleaned=0
    local failed=0
    local total_freed=0

    print_info "Starting cargo clean from: $start_dir"
    print_info "Searching for Cargo projects..."
    echo

    # Find all Cargo.toml files
    while IFS= read -r -d '' cargo_toml; do
        local project_dir=$(dirname "$cargo_toml")

        # Skip if this is a workspace member (workspace root will handle it)
        local parent_dir="$project_dir"
        while [ "$parent_dir" != "/" ] && [ "$parent_dir" != "." ]; do
            parent_dir=$(dirname "$parent_dir")
            if [ -f "$parent_dir/Cargo.toml" ] && grep -q "^\[workspace\]" "$parent_dir/Cargo.toml" 2>/dev/null; then
                print_info "Skipping workspace member: $project_dir"
                continue 2
            fi
        done

        # Clean the project
        if clean_project "$project_dir" "total_freed"; then
            ((cleaned++))
        else
            ((failed++))
        fi

        # Return to start directory
        cd "$start_dir" || exit 1

    done < <(find "$start_dir" -name "Cargo.toml" -type f -print0)

    # Summary
    echo
    print_info "=== SUMMARY ==="
    print_success "Successfully cleaned: $cleaned projects"

    if [ "$total_freed" -gt 0 ]; then
        local total_human=$(format_bytes "$total_freed")
        print_success "Total storage freed: $total_human"
    else
        print_info "No storage was freed"
    fi

    if [ $failed -gt 0 ]; then
        print_error "Failed to clean: $failed projects"
        return 1
    fi

    print_success "All done!"
}

# Show help
if [[ "$1" == "-h" || "$1" == "--help" ]]; then
    echo "Usage: $0 [directory]"
    echo
    echo "Recursively run 'cargo clean' in all directories containing Cargo.toml"
    echo "Default: current directory"
    exit 0
fi

# Run main function
main "$@"