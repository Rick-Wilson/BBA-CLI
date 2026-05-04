#!/bin/bash
#
# Build script for BBA-CLI on Windows via Parallels
# Run from macOS - uses Parallels shared folders
#
# SSH sessions don't inherit mapped drives, so we use net use to map them first.
#

set -e

# Load environment variables
source ~/.zshrc 2>/dev/null || true

VM_HOST="${WINDOWS_USER:-Rick}@${WINDOWS_HOST:-10.211.55.5}"
LOCAL_PROJECT_DIR="$(cd "$(dirname "$0")/.." && pwd)"

# Drive mapping: G: -> \\Mac\Home\Development\GitHub
DRIVE_MAP_CMD='net use G: \\Mac\Home\Development\GitHub >nul 2>&1'
WIN_PROJECT_DIR='G:\BBA-CLI'

# Parse command line arguments
BUILD_WRAPPER=true
BUILD_CLI=true

while [[ $# -gt 0 ]]; do
    case $1 in
        --wrapper-only)
            BUILD_CLI=false
            shift
            ;;
        --cli-only)
            BUILD_WRAPPER=false
            shift
            ;;
        -h|--help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --wrapper-only   Only build the C# wrapper"
            echo "  --cli-only       Only build the Rust CLI"
            echo "  -h, --help       Show this help message"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

echo "=============================================="
echo "BBA-CLI Windows Build (via Parallels)"
echo "=============================================="
echo "Local: $LOCAL_PROJECT_DIR"
echo "Windows: $WIN_PROJECT_DIR"
echo ""

# Step 1: Build C# wrapper on Windows
if $BUILD_WRAPPER; then
    echo "[1/3] Building C# EPBot wrapper..."

    ssh -T "$VM_HOST" "$DRIVE_MAP_CMD & cd /d $WIN_PROJECT_DIR\\epbot-wrapper && dotnet build -c Release"

    if [ $? -ne 0 ]; then
        echo "C# wrapper build failed"
        exit 1
    fi
else
    echo "[1/3] Skipping wrapper build (--cli-only)"
fi

# Step 2: Build Rust CLI on Windows
if $BUILD_CLI; then
    echo ""
    echo "[2/3] Building Rust CLI..."

    ssh -T "$VM_HOST" "$DRIVE_MAP_CMD & cd /d $WIN_PROJECT_DIR\\cli && cargo build --release"

    if [ $? -ne 0 ]; then
        echo "CLI build failed"
        exit 1
    fi
else
    echo "[2/3] Skipping CLI build (--wrapper-only)"
fi

# Step 3: Show results
echo ""
echo "[3/3] Build summary..."

echo ""
echo "=============================================="
echo "Build Complete"
echo "=============================================="
echo ""

if $BUILD_WRAPPER; then
    WRAPPER_EXE="$LOCAL_PROJECT_DIR/epbot-wrapper/bin/Release/net48/epbot-wrapper.exe"
    if [ -f "$WRAPPER_EXE" ]; then
        echo "C# Wrapper: epbot-wrapper/bin/Release/net48/epbot-wrapper.exe"
        ls -la "$WRAPPER_EXE"
    else
        echo "Note: C# wrapper not found (check build output above)"
    fi
fi

if $BUILD_CLI; then
    CLI_EXE="$LOCAL_PROJECT_DIR/cli/target/release/bba.exe"
    if [ -f "$CLI_EXE" ]; then
        echo "CLI executable: cli/target/release/bba.exe"
        ls -la "$CLI_EXE"
    else
        echo "Note: CLI executable not found (check build output above)"
    fi
fi

echo ""
echo "To test the C# wrapper on Windows:"
echo "  ssh $VM_HOST"
echo "  net use G: \\\\\\\\Mac\\\\Home\\\\Development\\\\GitHub"
echo "  cd G:\\BBA-CLI\\epbot-wrapper\\bin\\Release\\net48"
echo '  echo {"deals":[{"pbn":"N:AKQ.JT9.876.5432","dealer":"N","vulnerability":"None"}]} | epbot-wrapper.exe'
echo ""
