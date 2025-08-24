#!/bin/bash

# GCloud Convex Backend Detailed Restart Script

set -e

INSTANCE_NAME="convex-backend-dev"
ZONE="asia-northeast1-a"
DOCKER_PATH="/opt/convex"

echo "🔄 Restarting Convex Backend on GCloud instance: $INSTANCE_NAME"

# Check if instance is running
echo "📡 Checking instance status..."
INSTANCE_STATUS=$(gcloud compute instances describe $INSTANCE_NAME --zone=$ZONE --format="get(status)")
echo "Instance status: $INSTANCE_STATUS"

if [ "$INSTANCE_STATUS" != "RUNNING" ]; then
    echo "⚠️  Instance is not running. Starting instance first..."
    gcloud compute instances start $INSTANCE_NAME --zone=$ZONE
    echo "⏳ Waiting for instance to start..."
    sleep 30
fi

# Check if docker compose file exists and restart services
echo "🐳 Restarting Docker Compose services..."
gcloud compute ssh $INSTANCE_NAME \
  --zone=$ZONE \
  --command="
    set -e
    echo 'Checking docker compose file...'
    if [ -f 'docker-compose.yml' ]; then
        echo 'Current running services:'
        sudo docker-compose ps
        echo 'Restarting services...'
        sudo docker-compose restart
        echo 'Services after restart:'
        sudo docker-compose ps
        echo 'Checking logs for errors...'
        sudo docker-compose logs --tail=20
    else
        echo 'Error: docker-compose.yml not found in home directory'
        echo 'Available files:'
        ls -la
        exit 1
    fi
  "

echo "✅ Convex Backend restart completed successfully"
echo "🌐 You can check the status with:"
echo "   gcloud compute ssh $INSTANCE_NAME --zone=$ZONE --command='sudo docker-compose ps'"