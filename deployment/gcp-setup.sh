#!/bin/bash
# GCP Compute Engine Setup Script for Convex Backend

set -e

# 設定変数
PROJECT_ID="ai-sales-hub-dev-new"  # 現在のプロジェクトID
ZONE="asia-northeast1-a"  # 日本のリージョン
INSTANCE_NAME="convex-backend-instance"
MACHINE_TYPE="e2-standard-2"
DISK_SIZE="50GB"
DOMAIN="jbci-convex-dev.com"

echo "=== GCP Convex Backend Setup ==="

# プロジェクトIDの確認
if [ "$PROJECT_ID" = "your-gcp-project-id" ]; then
    echo "エラー: PROJECT_IDを実際のGCPプロジェクトIDに変更してください"
    exit 1
fi

# プロジェクトの設定
echo "Setting GCP project..."
gcloud config set project $PROJECT_ID

# ファイアウォールルールの作成
echo "Creating firewall rules..."
gcloud compute firewall-rules create allow-convex-ports \
  --allow tcp:3210,tcp:3211,tcp:6791,tcp:80,tcp:443 \
  --source-ranges 0.0.0.0/0 \
  --target-tags convex-server || echo "Firewall rule may already exist"

# Compute Engineインスタンスの作成
echo "Creating Compute Engine instance..."
gcloud compute instances create $INSTANCE_NAME \
  --zone=$ZONE \
  --machine-type=$MACHINE_TYPE \
  --boot-disk-size=$DISK_SIZE \
  --boot-disk-type=pd-ssd \
  --image-family=ubuntu-2204-lts \
  --image-project=ubuntu-os-cloud \
  --tags=convex-server \
  --metadata-from-file=startup-script=deployment/startup.sh

# 外部IPアドレスの取得
echo "Getting external IP address..."
EXTERNAL_IP=$(gcloud compute instances describe $INSTANCE_NAME \
  --zone=$ZONE --format="get(networkInterfaces[0].accessConfigs[0].natIP)")

echo "=== Setup Information ==="
echo "Instance Name: $INSTANCE_NAME"
echo "Zone: $ZONE"
echo "External IP: $EXTERNAL_IP"
echo ""
echo "DNS Records to create:"
echo "A record: $DOMAIN → $EXTERNAL_IP"
echo "A record: api.$DOMAIN → $EXTERNAL_IP"
echo "A record: dashboard.$DOMAIN → $EXTERNAL_IP"
echo ""
echo "SSH to instance:"
echo "gcloud compute ssh $INSTANCE_NAME --zone=$ZONE"
echo ""
echo "Setup logs:"
echo "gcloud compute instances get-serial-port-output $INSTANCE_NAME --zone=$ZONE"