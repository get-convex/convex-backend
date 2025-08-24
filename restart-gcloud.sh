#!/bin/bash

# GCloud Convex Backend Restart Script

set -e

echo "🔄 Restarting Convex Backend on GCloud..."

# Basic restart command (using docker-compose, not docker compose)
gcloud compute ssh convex-backend-dev \
  --zone=asia-northeast1-a \
  --command="sudo docker-compose restart"

echo "✅ Restart command sent to GCloud instance"