#!/bin/bash
# Convex Backend Deployment Script for GCP
# Ubuntu 20.04 LTS startup script

set -e

echo "=== Convex Backend Server Setup Started ==="

# システムの更新
echo "Updating system packages..."
apt-get update && apt-get upgrade -y

# Dockerのインストール
echo "Installing Docker..."
curl -fsSL https://get.docker.com | sh
usermod -aG docker $USER

# Docker Composeのインストール
echo "Installing Docker Compose..."
curl -L "https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose
chmod +x /usr/local/bin/docker-compose

# Nginxとcertbotのインストール
echo "Installing Nginx and Certbot..."
apt-get install -y nginx certbot python3-certbot-nginx

# 作業ディレクトリの作成
mkdir -p /opt/convex
cd /opt/convex

# docker-compose.ymlのダウンロード
echo "Downloading Convex docker-compose.yml..."
curl -O https://raw.githubusercontent.com/get-convex/convex-backend/main/self-hosted/docker/docker-compose.yml

# 環境変数ファイルの作成
echo "Creating environment configuration..."
cat > .env << EOF
CONVEX_CLOUD_ORIGIN=https://api.jbci-convex-dev.com
CONVEX_SITE_ORIGIN=https://jbci-convex-dev.com  
NEXT_PUBLIC_DEPLOYMENT_URL=https://api.jbci-convex-dev.com
EOF

# Nginxの設定
echo "Configuring Nginx..."
cat > /etc/nginx/sites-available/convex << 'NGINX_CONFIG'
# API endpoint
server {
    listen 80;
    server_name api.jbci-convex-dev.com;
    location / {
        proxy_pass http://localhost:3210;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}

# HTTP Actions
server {
    listen 80;
    server_name jbci-convex-dev.com;
    location / {
        proxy_pass http://localhost:3211;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}

# Dashboard
server {
    listen 80;
    server_name dashboard.jbci-convex-dev.com;
    location / {
        proxy_pass http://localhost:6791;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
NGINX_CONFIG

# Nginxサイトの有効化
ln -s /etc/nginx/sites-available/convex /etc/nginx/sites-enabled/
rm -f /etc/nginx/sites-enabled/default

# Nginx設定のテスト
nginx -t && systemctl reload nginx

# Docker Composeでサービス開始
echo "Starting Convex services..."
docker-compose up -d

# サービスの起動を待つ
echo "Waiting for services to start..."
sleep 30

# 管理キーの生成
echo "Generating admin key..."
ADMIN_KEY=$(docker-compose exec -T backend ./generate_admin_key.sh)
echo "Admin Key: $ADMIN_KEY" > /opt/convex/admin_key.txt

echo "=== Setup completed! ==="
echo "Admin key saved to: /opt/convex/admin_key.txt"
echo "Next steps:"
echo "1. Configure DNS to point to this server's IP"
echo "2. Run SSL certificate setup"
echo "3. Access dashboard at http://dashboard.jbci-convex-dev.com"