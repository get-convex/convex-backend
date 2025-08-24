#!/bin/bash
# Convex Self-Hosted on GCP Quick Setup Script
# 使用方法: ./quick-setup.sh your-domain.com your-email@example.com

set -e

# パラメータチェック
if [ $# -ne 2 ]; then
    echo "使用方法: $0 <domain> <email>"
    echo "例: $0 example.com admin@example.com"
    exit 1
fi

DOMAIN=$1
EMAIL=$2
PROJECT_ID=$(gcloud config get-value project)
ZONE="asia-northeast1-a"
INSTANCE_NAME="convex-backend-instance"

echo "=== Convex Self-Hosted GCP セットアップ ==="
echo "ドメイン: $DOMAIN"
echo "メール: $EMAIL"
echo "プロジェクト: $PROJECT_ID"
echo "ゾーン: $ZONE"
echo ""

# 確認
read -p "設定内容で実行しますか? (y/N): " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "セットアップをキャンセルしました。"
    exit 1
fi

echo "🚀 セットアップを開始します..."

# 1. ファイアウォールルールの作成
echo "📡 ファイアウォールルールを作成中..."
gcloud compute firewall-rules create allow-convex-ports \
  --allow tcp:3210,tcp:3211,tcp:6791,tcp:80,tcp:443 \
  --source-ranges 0.0.0.0/0 \
  --target-tags convex-server || echo "ファイアウォールルールは既に存在しています"

# 2. スタートアップスクリプトの生成
echo "📜 スタートアップスクリプトを生成中..."
cat > temp-startup.sh << EOF
#!/bin/bash
set -e

echo "=== Convex Backend Server Setup Started ==="

# システムの更新
apt-get update && apt-get upgrade -y

# Dockerのインストール
curl -fsSL https://get.docker.com | sh
usermod -aG docker \$USER

# Docker Composeのインストール
curl -L "https://github.com/docker/compose/releases/latest/download/docker-compose-\$(uname -s)-\$(uname -m)" -o /usr/local/bin/docker-compose
chmod +x /usr/local/bin/docker-compose

# Nginxとcertbotのインストール
apt-get install -y nginx certbot python3-certbot-nginx

# 作業ディレクトリの作成
mkdir -p /opt/convex
cd /opt/convex

# docker-compose.ymlのダウンロード
curl -O https://raw.githubusercontent.com/get-convex/convex-backend/main/self-hosted/docker/docker-compose.yml

# 環境変数ファイルの作成
cat > .env << ENVEOF
CONVEX_CLOUD_ORIGIN=https://api.${DOMAIN}
CONVEX_SITE_ORIGIN=https://${DOMAIN}
NEXT_PUBLIC_DEPLOYMENT_URL=https://api.${DOMAIN}
ENVEOF

# Nginxの設定（WebSocket対応）
cat > /etc/nginx/sites-available/convex << 'NGINX_CONFIG'
# API endpoint with WebSocket support
server {
    listen 80;
    server_name api.${DOMAIN};
    location / {
        proxy_pass http://localhost:3210;
        proxy_set_header Host \\\$host;
        proxy_set_header X-Real-IP \\\$remote_addr;
        proxy_set_header X-Forwarded-For \\\$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \\\$scheme;
        
        # WebSocket upgrade headers
        proxy_set_header Upgrade \\\$http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_http_version 1.1;
        proxy_cache_bypass \\\$http_upgrade;
        
        # Timeout settings for WebSocket connections
        proxy_connect_timeout 60s;
        proxy_send_timeout 60s;
        proxy_read_timeout 60s;
    }
}

# HTTP Actions
server {
    listen 80;
    server_name ${DOMAIN};
    location / {
        proxy_pass http://localhost:3211;
        proxy_set_header Host \\\$host;
        proxy_set_header X-Real-IP \\\$remote_addr;
        proxy_set_header X-Forwarded-For \\\$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \\\$scheme;
    }
}

# Dashboard
server {
    listen 80;
    server_name dashboard.${DOMAIN};
    location / {
        proxy_pass http://localhost:6791;
        proxy_set_header Host \\\$host;
        proxy_set_header X-Real-IP \\\$remote_addr;
        proxy_set_header X-Forwarded-For \\\$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \\\$scheme;
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
ADMIN_KEY=\$(docker-compose exec -T backend ./generate_admin_key.sh)
echo "Admin Key: \$ADMIN_KEY" > /opt/convex/admin_key.txt

echo "=== Setup completed! ==="
echo "Admin key saved to: /opt/convex/admin_key.txt"
echo "Next steps:"
echo "1. Configure DNS to point to this server's IP"
echo "2. Run SSL certificate setup"
echo "3. Access dashboard at http://dashboard.${DOMAIN}"
EOF

# 3. Compute Engineインスタンスの作成
echo "🖥️  Compute Engineインスタンスを作成中..."
gcloud compute instances create $INSTANCE_NAME \
  --zone=$ZONE \
  --machine-type=e2-standard-2 \
  --boot-disk-size=50GB \
  --boot-disk-type=pd-ssd \
  --image-family=ubuntu-2204-lts \
  --image-project=ubuntu-os-cloud \
  --tags=convex-server \
  --metadata-from-file=startup-script=temp-startup.sh

# 4. 外部IPアドレスの取得
echo "🌐 外部IPアドレスを取得中..."
sleep 10
EXTERNAL_IP=$(gcloud compute instances describe $INSTANCE_NAME \
  --zone=$ZONE --format="get(networkInterfaces[0].accessConfigs[0].natIP)")

echo "✅ インスタンス作成完了!"
echo "外部IP: $EXTERNAL_IP"

# 5. DNS設定のオプション表示
echo ""
echo "📋 次の手順:"
echo ""
echo "=== DNS設定が必要です ==="
echo "以下のDNSレコードを設定してください:"
echo ""
echo "| タイプ | 名前 | 値 |"
echo "|--------|------|-----|"
echo "| A | $DOMAIN | $EXTERNAL_IP |"
echo "| A | api.$DOMAIN | $EXTERNAL_IP |"
echo "| A | dashboard.$DOMAIN | $EXTERNAL_IP |"
echo ""

# 6. Google Cloud DNSの場合のコマンド生成
echo "=== Google Cloud DNSを使用する場合 ==="
cat > setup-dns.sh << EOF
#!/bin/bash
# DNS設定スクリプト
set -e

DOMAIN="$DOMAIN"
EXTERNAL_IP="$EXTERNAL_IP"

# DNS zone作成
gcloud dns managed-zones create convex-zone \\
  --dns-name=\$DOMAIN \\
  --description="DNS zone for Convex backend"

# DNSレコードの追加
gcloud dns record-sets transaction start --zone=convex-zone

gcloud dns record-sets transaction add \$EXTERNAL_IP \\
  --name=\$DOMAIN \\
  --ttl=300 \\
  --type=A \\
  --zone=convex-zone

gcloud dns record-sets transaction add \$EXTERNAL_IP \\
  --name=api.\$DOMAIN \\
  --ttl=300 \\
  --type=A \\
  --zone=convex-zone

gcloud dns record-sets transaction add \$EXTERNAL_IP \\
  --name=dashboard.\$DOMAIN \\
  --ttl=300 \\
  --type=A \\
  --zone=convex-zone

gcloud dns record-sets transaction execute --zone=convex-zone

echo "DNS設定完了!"
echo "ネームサーバー:"
gcloud dns managed-zones describe convex-zone --format="value(nameServers[])"
EOF

chmod +x setup-dns.sh

echo "Google Cloud DNSを使用する場合は: ./setup-dns.sh を実行"
echo ""

# 7. SSL設定のスクリプト生成
cat > setup-ssl.sh << EOF
#!/bin/bash
# SSL証明書設定スクリプト（DNS設定後に実行）
set -e

DOMAIN="$DOMAIN"
EMAIL="$EMAIL"
INSTANCE_NAME="$INSTANCE_NAME"
ZONE="$ZONE"

echo "DNS解決を確認中..."
nslookup \$DOMAIN
nslookup api.\$DOMAIN
nslookup dashboard.\$DOMAIN

echo "SSL証明書を取得中..."
gcloud compute ssh \$INSTANCE_NAME --zone=\$ZONE --command="
sudo certbot --nginx \\
  -d \$DOMAIN \\
  -d api.\$DOMAIN \\
  -d dashboard.\$DOMAIN \\
  --email \$EMAIL \\
  --agree-tos \\
  --non-interactive
"

echo "SSL設定完了!"
EOF

chmod +x setup-ssl.sh

echo "=== SSL証明書設定 ==="
echo "DNS設定が反映された後（24-48時間）、SSL証明書を設定:"
echo "./setup-ssl.sh"
echo ""

# 8. 管理キー取得スクリプト
cat > get-admin-key.sh << EOF
#!/bin/bash
# 管理キー取得スクリプト
set -e

INSTANCE_NAME="$INSTANCE_NAME"
ZONE="$ZONE"

echo "管理キーを取得中..."
gcloud compute ssh \$INSTANCE_NAME --zone=\$ZONE --command="
cd /opt/convex
sudo cat admin_key.txt
"
EOF

chmod +x get-admin-key.sh

echo "=== 管理キー取得 ==="
echo "セットアップ完了後、管理キーを取得:"
echo "./get-admin-key.sh"
echo ""

# 9. クリーンアップ
rm -f temp-startup.sh

echo "🎉 セットアップスクリプト実行完了!"
echo ""
echo "📝 作成されたファイル:"
echo "- setup-dns.sh (Google Cloud DNS設定用)"
echo "- setup-ssl.sh (SSL証明書設定用)"
echo "- get-admin-key.sh (管理キー取得用)"
echo ""
echo "⏰ 現在のステータス:"
echo "1. ✅ GCPインスタンス作成完了"
echo "2. ⏳ DNS設定が必要"
echo "3. ⏳ SSL証明書設定が必要"
echo ""
echo "📋 次の手順:"
echo "1. DNS設定を実行してください"
echo "2. DNS反映後（24-48時間）にSSL設定を実行"
echo "3. 管理キーを取得してダッシュボードにログイン"
echo ""
echo "🌐 アクセス先（DNS/SSL設定後）:"
echo "- Dashboard: https://dashboard.$DOMAIN"
echo "- API: https://api.$DOMAIN"
echo "- HTTP Actions: https://$DOMAIN"