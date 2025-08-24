# Google Cloud SQL Auth Proxy Setup

This directory contains scripts and configurations for setting up Google Cloud SQL Auth Proxy to connect to your Cloud SQL PostgreSQL instance.

## Files Overview

- `setup-cloudsql-service-account.sh` - Creates service account with proper IAM roles
- `install-cloudsql-proxy.sh` - Downloads and installs Cloud SQL Auth Proxy binary
- `docker-compose.cloudsql-proxy.yml` - Docker Compose configuration for proxy
- `README-cloudsql-proxy.md` - This documentation file

## Prerequisites

1. **Google Cloud CLI** - Install and authenticate with `gcloud auth login`
2. **Project Setup** - Set default project with `gcloud config set project YOUR_PROJECT_ID`
3. **APIs Enabled** - Cloud SQL Admin API and IAM API (scripts will enable automatically)
4. **Permissions** - Your account needs `resourcemanager.projects.setIamPolicy` permission

## Quick Start

### 1. Create Service Account

```bash
# Run the service account setup script
./setup-cloudsql-service-account.sh
```

This script will:
- Create a service account named `convex-cloudsql-proxy`
- Assign required IAM roles (`roles/cloudsql.client` and `roles/cloudsql.instanceUser`)
- Generate and download a service account key
- Set up secure key storage in `./keys/` directory

### 2. Install Cloud SQL Auth Proxy

```bash
# Install the Cloud SQL Auth Proxy binary
./install-cloudsql-proxy.sh
```

This script will:
- Detect your platform (Linux/macOS, AMD64/ARM64)
- Download the latest Cloud SQL Auth Proxy binary
- Install it to `/usr/local/bin` (or `~/bin` if no sudo access)
- Verify the installation

### 3. Connect to Your Database

#### Option A: Direct Binary Usage

```bash
# Connect with port forwarding
cloud-sql-proxy --credentials-file=./keys/convex-cloudsql-proxy-key.json \
                --port=5432 \
                YOUR_PROJECT_ID:asia-northeast1:convex-postgres

# Connect without port forwarding (Unix socket)
cloud-sql-proxy --credentials-file=./keys/convex-cloudsql-proxy-key.json \
                YOUR_PROJECT_ID:asia-northeast1:convex-postgres
```

#### Option B: Docker Compose

```bash
# Set environment variables
export PROJECT_ID=your-project-id
export REGION=asia-northeast1
export INSTANCE_NAME=convex-postgres

# Start the proxy container
docker-compose -f docker-compose.cloudsql-proxy.yml up -d

# Connect to PostgreSQL
psql -h localhost -p 5432 -U your-db-user -d your-database
```

## Required IAM Roles

The service account is granted the following roles:

### roles/cloudsql.client
- **Purpose**: Allows connection to Cloud SQL instances
- **Key Permission**: `cloudsql.instances.connect`
- **Required**: Yes - This is the minimum required role

### roles/cloudsql.instanceUser
- **Purpose**: Enables IAM database authentication
- **Key Permission**: `cloudsql.instances.login`
- **Required**: Optional - Only needed for IAM database authentication

## Security Best Practices

### Key Management
- Service account keys are stored in `./keys/` directory
- Directory permissions: `700` (owner read/write/execute only)
- Key file permissions: `600` (owner read/write only)
- Keys directory is automatically added to `.gitignore`

### Key Rotation
Rotate service account keys regularly:

```bash
# Delete old keys
gcloud iam service-accounts keys list \
    --iam-account=convex-cloudsql-proxy@YOUR_PROJECT_ID.iam.gserviceaccount.com

gcloud iam service-accounts keys delete KEY_ID \
    --iam-account=convex-cloudsql-proxy@YOUR_PROJECT_ID.iam.gserviceaccount.com

# Generate new key
gcloud iam service-accounts keys create ./keys/convex-cloudsql-proxy-key.json \
    --iam-account=convex-cloudsql-proxy@YOUR_PROJECT_ID.iam.gserviceaccount.com
```

### Network Security
- Use Cloud SQL Auth Proxy over SSL/TLS connections
- Restrict network access with authorized networks if needed
- Use private IP for Cloud SQL instances when possible

## Troubleshooting

### Common Issues

1. **403 Forbidden Errors**
   - Check that the service account has `roles/cloudsql.client` role
   - Verify the instance connection name format: `PROJECT:REGION:INSTANCE`
   - Ensure Cloud SQL Admin API is enabled

2. **Connection Timeout**
   - Verify the Cloud SQL instance is running
   - Check firewall rules and network connectivity
   - Confirm the instance connection name is correct

3. **Authentication Errors**
   - Ensure the service account key file exists and is readable
   - Check that the key file path is correct
   - Verify the service account has not been disabled

4. **Database Connection Issues**
   - Confirm database user exists and has proper permissions
   - Check that the database name is correct
   - Verify SSL settings match your Cloud SQL configuration

### Debugging Commands

```bash
# Check service account exists
gcloud iam service-accounts describe convex-cloudsql-proxy@YOUR_PROJECT_ID.iam.gserviceaccount.com

# List service account keys
gcloud iam service-accounts keys list \
    --iam-account=convex-cloudsql-proxy@YOUR_PROJECT_ID.iam.gserviceaccount.com

# Test Cloud SQL instance connectivity
gcloud sql instances describe convex-postgres

# Check IAM policy
gcloud projects get-iam-policy YOUR_PROJECT_ID \
    --flatten="bindings[].members" \
    --format="table(bindings.role)" \
    --filter="bindings.members:convex-cloudsql-proxy@YOUR_PROJECT_ID.iam.gserviceaccount.com"
```

## Environment Variables

Set these environment variables for easier management:

```bash
# Add to your ~/.bashrc or ~/.zshrc
export PROJECT_ID=your-project-id
export REGION=asia-northeast1
export INSTANCE_NAME=convex-postgres
export GOOGLE_APPLICATION_CREDENTIALS=./keys/convex-cloudsql-proxy-key.json
```

## Alternative Authentication Methods

### Default Service Account (GCE/GKE)
If running on Google Cloud Platform:

```bash
# Uses default service account credentials
cloud-sql-proxy YOUR_PROJECT_ID:asia-northeast1:convex-postgres
```

### OAuth 2.0 Token
For temporary access:

```bash
# Generate access token
TOKEN=$(gcloud auth print-access-token)

# Use token for authentication
cloud-sql-proxy --token=$TOKEN YOUR_PROJECT_ID:asia-northeast1:convex-postgres
```

## Connection String Examples

### PostgreSQL Connection Strings

```bash
# Direct connection through proxy
postgresql://username:password@localhost:5432/database_name

# Using Unix socket (when proxy runs without --port)
postgresql://username:password@/database_name?host=/cloudsql/PROJECT_ID:REGION:INSTANCE_NAME

# Environment variable format
DATABASE_URL=postgresql://username:password@localhost:5432/database_name
```

### Application Integration

```javascript
// Node.js example
const { Pool } = require('pg');

const pool = new Pool({
  user: 'your-db-user',
  host: 'localhost',
  database: 'your-database',
  password: 'your-password',
  port: 5432,
});
```

## Monitoring and Logging

### Cloud SQL Auth Proxy Logs
```bash
# Run proxy with verbose logging
cloud-sql-proxy --verbose \
                --credentials-file=./keys/convex-cloudsql-proxy-key.json \
                YOUR_PROJECT_ID:asia-northeast1:convex-postgres
```

### Cloud SQL Instance Logs
```bash
# View Cloud SQL logs
gcloud sql instances describe convex-postgres --format="value(settings.userLabels)"
```

## Support

For issues related to:
- **Cloud SQL Auth Proxy**: Check [official documentation](https://cloud.google.com/sql/docs/mysql/sql-proxy)
- **Cloud SQL**: Visit [Cloud SQL documentation](https://cloud.google.com/sql/docs/)
- **IAM Roles**: See [IAM documentation](https://cloud.google.com/iam/docs/)

## License

This configuration is provided as-is for the Convex backend project.