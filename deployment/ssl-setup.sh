#!/bin/bash
# SSL Certificate Setup Script for Convex Backend
# Run this on the server after DNS records are configured

set -e

DOMAIN="jbci-convex-dev.com"
EMAIL="your-email@example.com"  # 実際のメールアドレスに変更してください

echo "=== SSL Certificate Setup ==="

# メールアドレスの確認
if [ "$EMAIL" = "your-email@example.com" ]; then
    echo "エラー: EMAILを実際のメールアドレスに変更してください"
    exit 1
fi

# DNSの確認
echo "Checking DNS resolution..."
nslookup $DOMAIN
nslookup api.$DOMAIN
nslookup dashboard.$DOMAIN

echo "DNS records look good. Proceeding with SSL setup..."

# SSL証明書の取得
echo "Obtaining SSL certificates..."
certbot --nginx \
  -d $DOMAIN \
  -d api.$DOMAIN \
  -d dashboard.$DOMAIN \
  --email $EMAIL \
  --agree-tos \
  --non-interactive

# Nginx設定のリロード
systemctl reload nginx

# 自動更新の確認
echo "Setting up automatic certificate renewal..."
systemctl enable certbot.timer
systemctl start certbot.timer

echo "=== SSL Setup Complete ==="
echo "Certificates installed for:"
echo "- $DOMAIN"
echo "- api.$DOMAIN" 
echo "- dashboard.$DOMAIN"
echo ""
echo "Auto-renewal is enabled via systemd timer."