#!/bin/bash

# Convex Development Database Setup Script

set -e

INSTANCE_NAME="convex-postgres-dev"
DB_NAME="convex_dev"
USER_NAME="convex_dev_user"
PASSWORD=$(openssl rand -base64 32 | tr -d "=+/" | cut -c1-25)

echo "🚀 Setting up Convex development database..."

# Wait for instance to be ready
echo "⏳ Waiting for Cloud SQL instance to be ready..."
while true; do
    STATUS=$(gcloud sql instances describe $INSTANCE_NAME --format="get(state)" 2>/dev/null || echo "NOT_FOUND")
    if [ "$STATUS" = "RUNNABLE" ]; then
        echo "✅ Instance is ready!"
        break
    elif [ "$STATUS" = "NOT_FOUND" ]; then
        echo "❌ Instance not found. Please create it first."
        exit 1
    else
        echo "⏳ Instance status: $STATUS - waiting..."
        sleep 30
    fi
done

# Create database
echo "📦 Creating database: $DB_NAME"
gcloud sql databases create $DB_NAME --instance=$INSTANCE_NAME

# Create user
echo "👤 Creating user: $USER_NAME"
gcloud sql users create $USER_NAME --instance=$INSTANCE_NAME --password=$PASSWORD

# Grant privileges
echo "🔐 Granting privileges..."
gcloud sql users set-password postgres --instance=$INSTANCE_NAME --password=temp_admin_pass || true
gcloud sql connect $INSTANCE_NAME --user=postgres --database=$DB_NAME <<EOF || true
GRANT ALL PRIVILEGES ON DATABASE $DB_NAME TO $USER_NAME;
GRANT ALL PRIVILEGES ON SCHEMA public TO $USER_NAME;
ALTER USER $USER_NAME CREATEDB;
EOF

# Display connection info
echo ""
echo "✅ Development database setup complete!"
echo ""
echo "📋 Connection Details:"
echo "Instance: $INSTANCE_NAME"
echo "Database: $DB_NAME"  
echo "Username: $USER_NAME"
echo "Password: $PASSWORD"
echo ""
echo "🔗 Connection String:"
echo "postgresql://$USER_NAME:$PASSWORD@host.docker.internal:5432/$DB_NAME"
echo ""
echo "💾 Save these credentials securely!"

# Save to env file
cat > .env.dev <<EOF
# Convex Development Environment
POSTGRES_URL=postgresql://$USER_NAME:$PASSWORD@host.docker.internal:5432/$DB_NAME
DATABASE_URL=postgresql://$USER_NAME:$PASSWORD@host.docker.internal:5432/$DB_NAME
CONVEX_CLOUD_ORIGIN=http://localhost:3210
CONVEX_SITE_ORIGIN=http://localhost:3211
NEXT_PUBLIC_DEPLOYMENT_URL=http://localhost:3210
DO_NOT_REQUIRE_SSL=1
RUST_LOG=info
EOF

echo "📁 Development environment file created: .env.dev"