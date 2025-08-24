#!/bin/bash

# Google Cloud SQL Auth Proxy Service Account Setup Script
# This script creates a service account with the necessary permissions for Cloud SQL Auth Proxy

set -euo pipefail  # Exit on error, undefined variables, pipe failures

# Configuration
SERVICE_ACCOUNT_NAME="convex-cloudsql-proxy"
SERVICE_ACCOUNT_DISPLAY_NAME="Convex Cloud SQL Proxy Service Account"
INSTANCE_NAME="convex-postgres"
REGION="asia-northeast1"
KEY_FILE_NAME="${SERVICE_ACCOUNT_NAME}-key.json"
KEY_FILE_PATH="./keys/${KEY_FILE_NAME}"

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

# Check if gcloud is installed and authenticated
check_prerequisites() {
    log_info "Checking prerequisites..."
    
    if ! command -v gcloud &> /dev/null; then
        log_error "gcloud CLI is not installed. Please install it first."
        exit 1
    fi
    
    if ! gcloud auth list --filter=status:ACTIVE --format="value(account)" | grep -q .; then
        log_error "No active gcloud authentication found. Please run 'gcloud auth login' first."
        exit 1
    fi
    
    # Get current project
    PROJECT_ID=$(gcloud config get-value project 2>/dev/null)
    if [[ -z "$PROJECT_ID" ]]; then
        log_error "No default project set. Please run 'gcloud config set project YOUR_PROJECT_ID' first."
        exit 1
    fi
    
    log_info "Using project: $PROJECT_ID"
    
    # Check if Cloud SQL Admin API is enabled
    if ! gcloud services list --enabled --filter="name:sqladmin.googleapis.com" --format="value(name)" | grep -q .; then
        log_info "Enabling Cloud SQL Admin API..."
        gcloud services enable sqladmin.googleapis.com
        log_success "Cloud SQL Admin API enabled"
    fi
    
    # Check if IAM API is enabled
    if ! gcloud services list --enabled --filter="name:iam.googleapis.com" --format="value(name)" | grep -q .; then
        log_info "Enabling IAM API..."
        gcloud services enable iam.googleapis.com
        log_success "IAM API enabled"
    fi
}

# Check if service account already exists
check_service_account_exists() {
    if gcloud iam service-accounts describe "${SERVICE_ACCOUNT_NAME}@${PROJECT_ID}.iam.gserviceaccount.com" &>/dev/null; then
        log_warning "Service account ${SERVICE_ACCOUNT_NAME} already exists"
        read -p "Do you want to continue and update the existing service account? (y/N): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            log_info "Exiting without changes"
            exit 0
        fi
        return 0
    fi
    return 1
}

# Create service account
create_service_account() {
    log_info "Creating service account: ${SERVICE_ACCOUNT_NAME}"
    
    gcloud iam service-accounts create "$SERVICE_ACCOUNT_NAME" \
        --display-name "$SERVICE_ACCOUNT_DISPLAY_NAME" \
        --description "Service account for Cloud SQL Auth Proxy to connect to $INSTANCE_NAME"
    
    log_success "Service account created successfully"
}

# Assign IAM roles
assign_iam_roles() {
    log_info "Assigning IAM roles to service account..."
    
    SERVICE_ACCOUNT_EMAIL="${SERVICE_ACCOUNT_NAME}@${PROJECT_ID}.iam.gserviceaccount.com"
    
    # Cloud SQL Client role - required for Cloud SQL Auth Proxy
    log_info "Assigning Cloud SQL Client role..."
    gcloud projects add-iam-policy-binding "$PROJECT_ID" \
        --member="serviceAccount:$SERVICE_ACCOUNT_EMAIL" \
        --role="roles/cloudsql.client"
    
    # Optional: Cloud SQL Instance User role for IAM database authentication
    log_info "Assigning Cloud SQL Instance User role..."
    gcloud projects add-iam-policy-binding "$PROJECT_ID" \
        --member="serviceAccount:$SERVICE_ACCOUNT_EMAIL" \
        --role="roles/cloudsql.instanceUser"
    
    log_success "IAM roles assigned successfully"
}

# Create and secure key directory
setup_key_directory() {
    log_info "Setting up secure key directory..."
    
    if [[ ! -d "./keys" ]]; then
        mkdir -p "./keys"
        chmod 700 "./keys"
    fi
    
    # Add keys directory to .gitignore if it doesn't exist
    if [[ -f ".gitignore" ]]; then
        if ! grep -q "keys/" ".gitignore"; then
            echo "keys/" >> ".gitignore"
            log_info "Added keys/ to .gitignore"
        fi
    else
        echo "keys/" > ".gitignore"
        log_info "Created .gitignore with keys/ entry"
    fi
}

# Generate and download service account key
generate_service_account_key() {
    log_info "Generating service account key..."
    
    SERVICE_ACCOUNT_EMAIL="${SERVICE_ACCOUNT_NAME}@${PROJECT_ID}.iam.gserviceaccount.com"
    
    # Remove existing key file if it exists
    if [[ -f "$KEY_FILE_PATH" ]]; then
        log_warning "Existing key file found, removing..."
        rm "$KEY_FILE_PATH"
    fi
    
    # Generate new key
    gcloud iam service-accounts keys create "$KEY_FILE_PATH" \
        --iam-account="$SERVICE_ACCOUNT_EMAIL"
    
    # Secure the key file
    chmod 600 "$KEY_FILE_PATH"
    
    log_success "Service account key generated and saved to: $KEY_FILE_PATH"
}

# Verify Cloud SQL instance exists
verify_cloud_sql_instance() {
    log_info "Verifying Cloud SQL instance exists..."
    
    if gcloud sql instances describe "$INSTANCE_NAME" --quiet &>/dev/null; then
        log_success "Cloud SQL instance '$INSTANCE_NAME' found"
        
        # Get instance connection name
        CONNECTION_NAME=$(gcloud sql instances describe "$INSTANCE_NAME" --format="value(connectionName)")
        log_info "Instance connection name: $CONNECTION_NAME"
        
        return 0
    else
        log_warning "Cloud SQL instance '$INSTANCE_NAME' not found in project '$PROJECT_ID'"
        log_info "You may need to create the instance first or check the instance name"
        return 1
    fi
}

# Display usage instructions
display_usage_instructions() {
    log_success "Setup completed successfully!"
    echo
    echo "==== Usage Instructions ===="
    echo
    echo "1. Cloud SQL Auth Proxy connection command:"
    echo "   cloud-sql-proxy --credentials-file=${KEY_FILE_PATH} ${PROJECT_ID}:${REGION}:${INSTANCE_NAME}"
    echo
    echo "2. Alternative with port forwarding:"
    echo "   cloud-sql-proxy --credentials-file=${KEY_FILE_PATH} --port=5432 ${PROJECT_ID}:${REGION}:${INSTANCE_NAME}"
    echo
    echo "3. Environment variable setup:"
    echo "   export GOOGLE_APPLICATION_CREDENTIALS=${PWD}/${KEY_FILE_PATH}"
    echo
    echo "4. Docker usage:"
    echo "   docker run -d \\"
    echo "     -v ${PWD}/${KEY_FILE_PATH}:/key.json \\"
    echo "     -p 5432:5432 \\"
    echo "     gcr.io/cloudsql-docker/gce-proxy:1.33.2 \\"
    echo "     /cloud_sql_proxy -credential_file=/key.json \\"
    echo "     -instances=${PROJECT_ID}:${REGION}:${INSTANCE_NAME}=tcp:0.0.0.0:5432"
    echo
    echo "==== Security Notes ===="
    echo "- Key file location: ${KEY_FILE_PATH}"
    echo "- Key file permissions: 600 (read-write for owner only)"
    echo "- Keys directory added to .gitignore"
    echo "- Rotate keys regularly for security"
    echo "- Never commit service account keys to version control"
    echo
    echo "Service Account Email: ${SERVICE_ACCOUNT_NAME}@${PROJECT_ID}.iam.gserviceaccount.com"
}

# Main execution
main() {
    log_info "Starting Google Cloud SQL Auth Proxy Service Account Setup"
    echo "============================================================="
    
    check_prerequisites
    
    # Check if service account already exists
    SA_EXISTS=false
    if check_service_account_exists; then
        SA_EXISTS=true
    fi
    
    # Create service account if it doesn't exist
    if [[ "$SA_EXISTS" = false ]]; then
        create_service_account
    fi
    
    # Always assign roles (in case they were missing)
    assign_iam_roles
    
    # Setup key directory
    setup_key_directory
    
    # Generate new key
    generate_service_account_key
    
    # Verify instance exists (optional check)
    verify_cloud_sql_instance
    
    # Display usage instructions
    display_usage_instructions
    
    log_success "All operations completed successfully!"
}

# Handle script interruption
cleanup() {
    log_warning "Script interrupted. Cleaning up..."
    if [[ -f "$KEY_FILE_PATH" && ! -s "$KEY_FILE_PATH" ]]; then
        rm -f "$KEY_FILE_PATH"
        log_info "Removed incomplete key file"
    fi
    exit 1
}

trap cleanup INT TERM

# Run main function
main "$@"