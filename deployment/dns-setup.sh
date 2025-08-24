#!/bin/bash
# DNS Setup Script for Google Cloud DNS
# Run this after obtaining the server IP address

set -e

DOMAIN="jbci-convex-dev.com"
DNS_ZONE_NAME="convex-zone"
PROJECT_ID="ai-sales-hub-dev-new"  # 現在のプロジェクトID
SERVER_IP="34.84.75.217"  # インスタンスの外部IP

echo "=== DNS Setup for $DOMAIN ==="

# 設定の確認
if [ "$PROJECT_ID" = "your-gcp-project-id" ]; then
    echo "エラー: PROJECT_IDを実際のGCPプロジェクトIDに変更してください"
    exit 1
fi

if [ "$SERVER_IP" = "YOUR_SERVER_IP" ]; then
    echo "エラー: SERVER_IPを実際のサーバーIPアドレスに変更してください"
    exit 1
fi

# プロジェクトの設定
gcloud config set project $PROJECT_ID

# Cloud DNSゾーンの作成
echo "Creating Cloud DNS zone..."
gcloud dns managed-zones create $DNS_ZONE_NAME \
  --dns-name=$DOMAIN \
  --description="DNS zone for Convex backend" || echo "Zone may already exist"

# Aレコードの追加
echo "Adding DNS records..."

# メインドメイン
gcloud dns record-sets transaction start --zone=$DNS_ZONE_NAME
gcloud dns record-sets transaction add $SERVER_IP \
  --name=$DOMAIN \
  --ttl=300 \
  --type=A \
  --zone=$DNS_ZONE_NAME || true

# APIサブドメイン
gcloud dns record-sets transaction add $SERVER_IP \
  --name=api.$DOMAIN \
  --ttl=300 \
  --type=A \
  --zone=$DNS_ZONE_NAME || true

# ダッシュボードサブドメイン
gcloud dns record-sets transaction add $SERVER_IP \
  --name=dashboard.$DOMAIN \
  --ttl=300 \
  --type=A \
  --zone=$DNS_ZONE_NAME || true

gcloud dns record-sets transaction execute --zone=$DNS_ZONE_NAME

# ネームサーバーの表示
echo "=== DNS Setup Complete ==="
echo "Name servers for $DOMAIN:"
gcloud dns managed-zones describe $DNS_ZONE_NAME --format="value(nameServers[0])"
gcloud dns managed-zones describe $DNS_ZONE_NAME --format="value(nameServers[1])"
gcloud dns managed-zones describe $DNS_ZONE_NAME --format="value(nameServers[2])"
gcloud dns managed-zones describe $DNS_ZONE_NAME --format="value(nameServers[3])"
echo ""
echo "Configure these name servers in your domain registrar."
echo "DNS propagation may take up to 48 hours."