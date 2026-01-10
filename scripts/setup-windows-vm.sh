#!/bin/bash
#
# Setup script for Windows VM build environment
# Run from macOS: ./scripts/setup-windows-vm.sh
#
# This script installs the required tools on the Windows VM:
# - Rust (via rustup)
# - Visual Studio Build Tools with C++/CLI support
#

set -e

# Load environment variables
source ~/.zshrc 2>/dev/null || true

VM_HOST="${WINDOWS_USER:-Rick}@${WINDOWS_HOST:-10.211.55.5}"
VM_PROJECT_DIR="C:/Users/${WINDOWS_USER:-Rick}/Development/BBA-CLI"

echo "=============================================="
echo "BBA-CLI Windows VM Setup"
echo "=============================================="
echo "VM Host: $VM_HOST"
echo "Project Dir: $VM_PROJECT_DIR"
echo ""

# Test SSH connection
echo "[1/5] Testing SSH connection..."
if ! ssh "$VM_HOST" "echo Connection successful" 2>/dev/null; then
    echo "ERROR: Cannot connect to Windows VM via SSH"
    echo "Please ensure:"
    echo "  1. Windows VM is running"
    echo "  2. OpenSSH Server is enabled on Windows"
    echo "  3. SSH key authentication is configured"
    exit 1
fi

# Create project directory
echo "[2/5] Creating project directory..."
ssh "$VM_HOST" "if not exist \"$VM_PROJECT_DIR\" mkdir \"$VM_PROJECT_DIR\""

# Check/Install Rust
echo "[3/5] Checking Rust installation..."
ssh "$VM_HOST" << 'RUSTCHECK'
@echo off
where rustc >nul 2>nul
if %ERRORLEVEL% NEQ 0 (
    echo Rust not found. Installing via rustup...
    echo Please follow the prompts in the Windows VM to complete Rust installation.
    echo.
    echo Run this command on Windows:
    echo   winget install Rustlang.Rustup
    echo.
    echo Or download from: https://rustup.rs
    exit /b 1
) else (
    echo Rust is installed:
    rustc --version
    cargo --version
)
RUSTCHECK

# Check/Install Visual Studio Build Tools
echo "[4/5] Checking Visual Studio Build Tools..."
ssh "$VM_HOST" << 'VSCHECK'
@echo off
where cl >nul 2>nul
if %ERRORLEVEL% NEQ 0 (
    echo Visual Studio Build Tools not found in PATH.
    echo.
    echo Please install Visual Studio Build Tools with C++/CLI support:
    echo   1. Download from: https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022
    echo   2. Select "Desktop development with C++" workload
    echo   3. In Individual Components, also select:
    echo      - C++/CLI support for v143 build tools
    echo      - .NET Framework 4.8 targeting pack
    echo.
    exit /b 1
) else (
    echo Visual Studio Build Tools found:
    cl 2>&1 | findstr /C:"Version"
)
VSCHECK

# Create convenience scripts on Windows
echo "[5/5] Creating convenience scripts on Windows..."
ssh "$VM_HOST" << SCRIPTS
@echo off
echo Creating build scripts...

echo @echo off > "$VM_PROJECT_DIR/build-wrapper.bat"
echo call "C:\Program Files\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >> "$VM_PROJECT_DIR/build-wrapper.bat"
echo cd /d "$VM_PROJECT_DIR\wrapper" >> "$VM_PROJECT_DIR/build-wrapper.bat"
echo cmake -B build -G "Visual Studio 17 2022" -A x64 >> "$VM_PROJECT_DIR/build-wrapper.bat"
echo cmake --build build --config Release >> "$VM_PROJECT_DIR/build-wrapper.bat"

echo @echo off > "$VM_PROJECT_DIR/build-cli.bat"
echo cd /d "$VM_PROJECT_DIR\cli" >> "$VM_PROJECT_DIR/build-cli.bat"
echo cargo build --release >> "$VM_PROJECT_DIR/build-cli.bat"

echo @echo off > "$VM_PROJECT_DIR/build-all.bat"
echo call "$VM_PROJECT_DIR\build-wrapper.bat" >> "$VM_PROJECT_DIR/build-all.bat"
echo call "$VM_PROJECT_DIR\build-cli.bat" >> "$VM_PROJECT_DIR/build-all.bat"

echo Done creating scripts.
SCRIPTS

echo ""
echo "=============================================="
echo "Setup Summary"
echo "=============================================="
echo ""
echo "The Windows VM needs the following tools installed:"
echo ""
echo "1. Rust (via rustup):"
echo "   winget install Rustlang.Rustup"
echo "   (or download from https://rustup.rs)"
echo ""
echo "2. Visual Studio Build Tools 2022:"
echo "   - Download from: https://visualstudio.microsoft.com/downloads/"
echo "   - Install 'Desktop development with C++' workload"
echo "   - Add 'C++/CLI support for v143 build tools' component"
echo "   - Add '.NET Framework 4.8 targeting pack' component"
echo ""
echo "After installing these tools, run ./scripts/build-windows.sh"
echo ""
