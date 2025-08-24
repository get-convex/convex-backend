# Convex Self-Hosted on Google Cloud Platform 完全ガイド

このガイドでは、Google Cloud PlatformでConvexバックエンドをセルフホスティングする手順を詳しく説明します。

## 📋 前提条件

- GCPアカウントとプロジェクトが作成済み
- gcloud CLIがインストール・認証済み
- ドメインの管理権限（Google Cloud Domainsまたは外部レジストラー）
- 基本的なDocker、Nginx、SSL証明書の知識

## 🛠️ 必要なリソース

### 推奨スペック
- **マシンタイプ**: e2-standard-2 (2 vCPU, 8GB RAM)
- **ディスク**: 50GB SSD
- **OS**: Ubuntu 22.04 LTS
- **リージョン**: asia-northeast1 (東京)

### 必要なポート
- 80, 443 (HTTP/HTTPS)
- 3210 (Convex API)
- 3211 (Convex HTTP Actions)
- 6791 (Convex Dashboard)

## 🚀 セットアップ手順

### 1. ドメインの取得

#### Option A: Google Cloud Domains
```bash
# ドメイン検索
gcloud domains registrations search-domains "your-domain.com"

# 連絡先情報ファイルの作成
cat > contact.yaml << EOF
registrantContact:
  email: "your-email@example.com"
  phoneNumber: "+81.312345678"
  postalAddress:
    addressLines: ["Your Address"]
    administrativeArea: "Tokyo"
    locality: "Tokyo"
    organization: "Your Organization"
    postalCode: "100-0001"
    recipients: ["Your Name"]
    regionCode: "JP"
adminContact:
  # 同じ内容
technicalContact:
  # 同じ内容
EOF

# ドメイン登録
gcloud domains registrations register your-domain.com \
  --contact-data-from-file=contact.yaml \
  --contact-privacy=redacted-contact-data \
  --yearly-price="12.00 USD" \
  --name-servers=ns-cloud-e1.googledomains.com,ns-cloud-e2.googledomains.com,ns-cloud-e3.googledomains.com,ns-cloud-e4.googledomains.com
```

#### Option B: 外部ドメインレジストラー
お名前.com、ムームードメインなどで取得し、後でGoogle Cloud DNSに設定

### 2. GCP インフラストラクチャのセットアップ

#### プロジェクト設定
```bash
export PROJECT_ID="your-gcp-project-id"
export ZONE="asia-northeast1-a"
export INSTANCE_NAME="convex-backend-instance"
export DOMAIN="your-domain.com"

gcloud config set project $PROJECT_ID
```

#### ファイアウォールルールの作成
```bash
gcloud compute firewall-rules create allow-convex-ports \
  --allow tcp:3210,tcp:3211,tcp:6791,tcp:80,tcp:443 \
  --source-ranges 0.0.0.0/0 \
  --target-tags convex-server
```

#### スタートアップスクリプトの作成
```bash
cat > startup.sh << 'EOF'
#!/bin/bash
set -e

echo "=== Convex Backend Server Setup Started ==="

# システムの更新
apt-get update && apt-get upgrade -y

# Dockerのインストール
curl -fsSL https://get.docker.com | sh
usermod -aG docker $USER

# Docker Composeのインストール
curl -L "https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose
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
    server_name api.DOMAIN_PLACEHOLDER;
    location / {
        proxy_pass http://localhost:3210;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        
        # WebSocket upgrade headers
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_http_version 1.1;
        proxy_cache_bypass $http_upgrade;
        
        # Timeout settings for WebSocket connections
        proxy_connect_timeout 60s;
        proxy_send_timeout 60s;
        proxy_read_timeout 60s;
    }
}

# HTTP Actions
server {
    listen 80;
    server_name DOMAIN_PLACEHOLDER;
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
    server_name dashboard.DOMAIN_PLACEHOLDER;
    location / {
        proxy_pass http://localhost:6791;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
NGINX_CONFIG

# ドメイン名を置換
sed -i "s/DOMAIN_PLACEHOLDER/${DOMAIN}/g" /etc/nginx/sites-available/convex

# Nginxサイトの有効化
ln -s /etc/nginx/sites-available/convex /etc/nginx/sites-enabled/
rm -f /etc/nginx/sites-enabled/default

# Nginx設定のテスト
nginx -t && systemctl reload nginx

echo "=== Setup completed! ==="
EOF

chmod +x startup.sh
```

#### Compute Engineインスタンスの作成
```bash
gcloud compute instances create $INSTANCE_NAME \
  --zone=$ZONE \
  --machine-type=e2-standard-2 \
  --boot-disk-size=50GB \
  --boot-disk-type=pd-ssd \
  --image-family=ubuntu-2204-lts \
  --image-project=ubuntu-os-cloud \
  --tags=convex-server \
  --metadata-from-file=startup-script=startup.sh
```

#### 外部IPアドレスの取得
```bash
EXTERNAL_IP=$(gcloud compute instances describe $INSTANCE_NAME \
  --zone=$ZONE --format="get(networkInterfaces[0].accessConfigs[0].natIP)")
echo "External IP: $EXTERNAL_IP"
```

### 3. DNS設定

#### Google Cloud DNSの場合
```bash
# DNS zone作成
gcloud dns managed-zones create convex-zone \
  --dns-name=$DOMAIN \
  --description="DNS zone for Convex backend"

# DNSレコードの追加
gcloud dns record-sets transaction start --zone=convex-zone

gcloud dns record-sets transaction add $EXTERNAL_IP \
  --name=$DOMAIN \
  --ttl=300 \
  --type=A \
  --zone=convex-zone

gcloud dns record-sets transaction add $EXTERNAL_IP \
  --name=api.$DOMAIN \
  --ttl=300 \
  --type=A \
  --zone=convex-zone

gcloud dns record-sets transaction add $EXTERNAL_IP \
  --name=dashboard.$DOMAIN \
  --ttl=300 \
  --type=A \
  --zone=convex-zone

gcloud dns record-sets transaction execute --zone=convex-zone

# ドメインをCloud DNS zonに設定
gcloud domains registrations configure dns $DOMAIN \
  --cloud-dns-zone=convex-zone
```

#### 外部ドメインレジストラーの場合
以下のDNSレコードを設定：
| タイプ | 名前 | 値 |
|--------|------|-----|
| A | your-domain.com | [EXTERNAL_IP] |
| A | api.your-domain.com | [EXTERNAL_IP] |
| A | dashboard.your-domain.com | [EXTERNAL_IP] |

### 4. Convexサービスの起動

```bash
# サーバーにSSH接続
gcloud compute ssh $INSTANCE_NAME --zone=$ZONE

# Convexサービスの起動
cd /opt/convex
sudo docker-compose up -d

# サービス状態の確認
sudo docker-compose ps
sudo docker-compose logs backend --tail=20
```

### 5. SSL証明書の設定

DNS設定が反映された後（通常24-48時間以内）：

```bash
# DNS解決の確認
nslookup $DOMAIN
nslookup api.$DOMAIN
nslookup dashboard.$DOMAIN

# SSL証明書の取得
sudo certbot --nginx \
  -d $DOMAIN \
  -d api.$DOMAIN \
  -d dashboard.$DOMAIN \
  --email your-email@example.com \
  --agree-tos \
  --non-interactive

# Nginx設定のリロード
sudo systemctl reload nginx
```

### 6. 管理キーの生成

```bash
cd /opt/convex
sudo docker-compose exec -T backend ./generate_admin_key.sh | sudo tee admin_key.txt
cat admin_key.txt
```

## 🎯 アクセス先とテスト

### エンドポイント
- **API**: https://api.your-domain.com
- **Dashboard**: https://dashboard.your-domain.com
- **HTTP Actions**: https://your-domain.com

### 接続テスト
```bash
# API の接続テスト
curl https://api.your-domain.com/version

# 管理キーのテスト
curl -X GET "https://api.your-domain.com/api/check_admin_key" \
  -H "Authorization: Convex [ADMIN_KEY]" \
  -H "Content-Type: application/json"

# WebSocket接続のテスト
curl -X GET "https://api.your-domain.com/api/1.25.1/sync" \
  -H "Connection: Upgrade" \
  -H "Upgrade: websocket" \
  -H "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==" \
  -H "Sec-WebSocket-Version: 13" \
  -v
```

### ダッシュボードログイン
1. https://dashboard.your-domain.com にアクセス
2. 生成された管理キーを入力
3. デプロイメントがオンラインになることを確認

## 💻 ローカル開発環境の設定

`.env.local`ファイルに以下を追加：
```bash
CONVEX_SELF_HOSTED_URL='https://api.your-domain.com'
CONVEX_SELF_HOSTED_ADMIN_KEY='[管理キー]'
```

Convex CLIの使用：
```bash
npm install convex@latest
npx convex dev
npx convex deploy
```

## 🔧 トラブルシューティング

### よくある問題と解決法

#### 1. "This deployment is not online" エラー
- **原因**: WebSocket接続の問題
- **解決**: Nginx設定にWebSocketヘッダーが含まれているか確認
```bash
# nginx設定を確認
sudo cat /etc/nginx/sites-enabled/convex | grep -A 5 "Upgrade"
```

#### 2. SSL証明書エラー
- **原因**: DNS設定の未反映またはファイアウォール
- **解決**: DNS propagationの確認とポート80/443の開放
```bash
# DNS確認
nslookup your-domain.com
# ポート確認
sudo netstat -tlnp | grep :443
```

#### 3. Docker Composeサービスが起動しない
- **原因**: ポート競合または設定ファイルエラー
- **解決**: ログの確認とポート使用状況チェック
```bash
sudo docker-compose logs
sudo ss -tlnp | grep :3210
```

#### 4. 管理キーが無効
- **原因**: キー生成エラーまたはバックエンドの再起動
- **解決**: 新しいキーの生成
```bash
sudo docker-compose exec -T backend ./generate_admin_key.sh
```

### ログの確認方法

```bash
# インスタンスのシリアルログ
gcloud compute instances get-serial-port-output $INSTANCE_NAME --zone=$ZONE

# Dockerサービスログ
sudo docker-compose logs -f backend
sudo docker-compose logs -f dashboard

# Nginxログ
sudo tail -f /var/log/nginx/access.log
sudo tail -f /var/log/nginx/error.log

# システムログ
sudo journalctl -u nginx -f
```

## 📊 運用・メンテナンス

### 定期バックアップ
```bash
# データのエクスポート
npx convex export --path backup-$(date +%Y%m%d).zip

# 環境変数のバックアップ
npx convex env list > env-backup.txt
```

### アップデート
```bash
# Dockerイメージの更新
sudo docker-compose pull
sudo docker-compose up -d

# SSL証明書の自動更新確認
sudo systemctl status certbot.timer
```

### モニタリング
```bash
# サービス状態の確認
sudo docker-compose ps
sudo systemctl status nginx

# リソース使用状況
htop
df -h
```

## ⚡ パフォーマンス最適化

### 推奨設定
- **データベース**: 本番環境では外部PostgreSQL/MySQLを使用
- **ストレージ**: S3互換ストレージの使用を検討
- **CDN**: CloudflareやCloud CDNの設定
- **バックアップ**: 定期的なスナップショット作成

### スケーリング
- マシンタイプのアップグレード
- ロードバランサーの設定
- 複数リージョンでのデプロイ

## 🔒 セキュリティ

### 推奨事項
- 管理キーの安全な保管
- 定期的なセキュリティアップデート
- ファイアウォールの適切な設定
- SSL証明書の自動更新設定

### ネットワークセキュリティ
```bash
# 不要なポートの閉鎖
sudo ufw enable
sudo ufw allow 22,80,443/tcp

# fail2banの設定
sudo apt install fail2ban
```

## 📝 参考リンク

- [Convex Self-hosted Documentation](https://docs.convex.dev/production/hosting/self-hosted)
- [Google Cloud Compute Engine](https://cloud.google.com/compute)
- [Google Cloud DNS](https://cloud.google.com/dns)
- [Let's Encrypt](https://letsencrypt.org/)
- [Nginx WebSocket Proxying](https://nginx.org/en/docs/http/websocket.html)

---

このガイドに従うことで、Google Cloud Platform上でConvexバックエンドを確実にセルフホスティングできます。問題が発生した場合は、トラブルシューティングセクションを参照してください。