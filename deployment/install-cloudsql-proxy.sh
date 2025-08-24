#!/bin/bash

# Cloud SQL Auth Proxy Installation Script
# Downloads and installs the latest Cloud SQL Auth Proxy binary

set -euo pipefail

# Configuration
PROXY_VERSION="v2.17.1"  # Latest stable version as of 2025
INSTALL_DIR="/usr/local/bin"
TEMP_DIR="/tmp/cloudsql-proxy-install"

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Detect OS and architecture
detect_platform() {
    OS=$(uname -s | tr '[:upper:]' '[:lower:]')
    ARCH=$(uname -m)
    
    case $OS in
        linux*)
            PLATFORM="linux"
            ;;
        darwin*)
            PLATFORM="darwin"
            ;;
        *)
            log_error "Unsupported operating system: $OS"
            exit 1
            ;;
    esac
    
    case $ARCH in
        x86_64)
            ARCH="amd64"
            ;;
        arm64|aarch64)
            ARCH="arm64"
            ;;
        *)
            log_error "Unsupported architecture: $ARCH"
            exit 1
            ;;
    esac
    
    log_info "Detected platform: $PLATFORM-$ARCH"
}

# Check if running as root for system-wide installation
check_permissions() {
    if [[ "$INSTALL_DIR" == "/usr/local/bin" ]]; then
        if [[ $EUID -ne 0 ]]; then
            log_warning "System-wide installation requires root privileges"
            log_info "You can either:"
            log_info "1. Run this script with sudo"
            log_info "2. Install to a user directory (e.g., ~/bin)"
            read -p "Install to ~/bin instead? (y/N): " -n 1 -r
            echo
            if [[ $REPLY =~ ^[Yy]$ ]]; then
                INSTALL_DIR="$HOME/bin"
                mkdir -p "$INSTALL_DIR"
                
                # Check if ~/bin is in PATH
                if [[ ":$PATH:" != *":$HOME/bin:"* ]]; then
                    log_warning "~/bin is not in your PATH"
                    log_info "Add this line to your ~/.bashrc or ~/.zshrc:"
                    log_info "export PATH=\"\$HOME/bin:\$PATH\""
                fi
            else
                log_error "Please run with sudo or choose a different installation directory"
                exit 1
            fi
        fi
    fi
}

# Download Cloud SQL Auth Proxy
download_proxy() {
    log_info "Creating temporary directory..."
    mkdir -p "$TEMP_DIR"
    cd "$TEMP_DIR"
    
    # Construct download URL
    BINARY_NAME="cloud-sql-proxy.${PLATFORM}.${ARCH}"
    DOWNLOAD_URL="https://storage.googleapis.com/cloud-sql-connectors/cloud-sql-proxy/${PROXY_VERSION}/${BINARY_NAME}"
    
    log_info "Downloading Cloud SQL Auth Proxy $PROXY_VERSION..."
    log_info "URL: $DOWNLOAD_URL"
    
    if command -v curl &> /dev/null; then
        curl -fsSL "$DOWNLOAD_URL" -o "cloud-sql-proxy" || {
            log_error "Failed to download Cloud SQL Auth Proxy"
            exit 1
        }
    elif command -v wget &> /dev/null; then
        wget -q "$DOWNLOAD_URL" -O "cloud-sql-proxy" || {
            log_error "Failed to download Cloud SQL Auth Proxy"
            exit 1
        }
    else
        log_error "Neither curl nor wget is available"
        exit 1
    fi
    
    log_success "Downloaded Cloud SQL Auth Proxy binary"
}

# Install the binary
install_proxy() {
    log_info "Installing Cloud SQL Auth Proxy to $INSTALL_DIR..."
    
    # Make binary executable
    chmod +x cloud-sql-proxy
    
    # Move to installation directory
    mv cloud-sql-proxy "$INSTALL_DIR/"
    
    log_success "Cloud SQL Auth Proxy installed successfully"
}

# Verify installation
verify_installation() {
    log_info "Verifying installation..."
    
    if command -v cloud-sql-proxy &> /dev/null; then
        VERSION_OUTPUT=$(cloud-sql-proxy --version 2>&1 || echo "Version check failed")
        log_success "Cloud SQL Auth Proxy is installed and accessible"
        log_info "Version: $VERSION_OUTPUT"
    else
        log_error "Cloud SQL Auth Proxy is not in PATH"
        log_info "Binary location: $INSTALL_DIR/cloud-sql-proxy"
        
        if [[ "$INSTALL_DIR" != "/usr/local/bin" ]]; then
            log_info "Make sure $INSTALL_DIR is in your PATH"
        fi
    fi
}

# Display usage examples
display_usage() {
    log_info "Basic usage examples:"
    echo
    echo "1. Connect with service account key:"
    echo "   cloud-sql-proxy --credentials-file=path/to/key.json PROJECT:REGION:INSTANCE"
    echo
    echo "2. Connect with port forwarding:"
    echo "   cloud-sql-proxy --port=5432 PROJECT:REGION:INSTANCE"
    echo
    echo "3. Connect with IAM authentication:"
    echo "   cloud-sql-proxy --auto-iam-authn PROJECT:REGION:INSTANCE"
    echo
    echo "4. Get help:"
    echo "   cloud-sql-proxy --help"
    echo
    echo "For your specific instance:"
    echo "   cloud-sql-proxy --credentials-file=./keys/convex-cloudsql-proxy-key.json \\"
    echo "                   --port=5432 \\"
    echo "                   YOUR_PROJECT:asia-northeast1:convex-postgres"
}

# Cleanup function
cleanup() {
    log_info "Cleaning up temporary files..."
    rm -rf "$TEMP_DIR"
}

# Main execution
main() {
    log_info "Starting Cloud SQL Auth Proxy installation"
    echo "============================================="
    
    detect_platform
    check_permissions
    download_proxy
    install_proxy
    verify_installation
    display_usage
    
    log_success "Installation completed successfully!"
}

# Handle script interruption
trap cleanup INT TERM EXIT

# Run main function
main "$@"