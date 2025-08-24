#!/bin/bash
# Google Cloud SQL for Convex Setup Script
# 使用方法: ./cloud-sql-setup.sh

set -e

# パラメータ設定
PROJECT_ID=$(gcloud config get-value project)
REGION="asia-northeast1"
INSTANCE_NAME="convex-postgres"
DB_NAME="convex_self_hosted"
DB_USER="convex_user"
DB_PASSWORD="$(openssl rand -base64 32)"

echo "=== Google Cloud SQL for Convex Setup ==="
echo "プロジェクト: $PROJECT_ID"
echo "リージョン: $REGION"
echo "インスタンス名: $INSTANCE_NAME"
echo "データベース名: $DB_NAME"
echo "ユーザー名: $DB_USER"
echo "パスワード: $DB_PASSWORD"
echo ""

# 確認
read -p "設定を確認しました。続行しますか? (y/N): " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "セットアップをキャンセルしました。"
    exit 1
fi

# Cloud SQL API の有効化
echo "🔌 Cloud SQL API を有効化中..."
gcloud services enable sqladmin.googleapis.com
gcloud services enable servicenetworking.googleapis.com

# インスタンスの作成
echo "🗄️  Cloud SQL インスタンスを作成中..."
gcloud sql instances create $INSTANCE_NAME \
  --database-version=POSTGRES_15 \
  --tier=db-custom-2-4096 \
  --region=$REGION \
  --storage-type=SSD \
  --storage-size=100GB \
  --storage-auto-increase \
  --backup-start-time=03:00 \
  --maintenance-window-day=SUN \
  --maintenance-window-hour=04

# データベースとユーザーの作成
echo "👤 データベースとユーザーを作成中..."
gcloud sql databases create $DB_NAME --instance=$INSTANCE_NAME
gcloud sql users create $DB_USER --instance=$INSTANCE_NAME --password=$DB_PASSWORD

# プライベートIP設定
echo "🔒 プライベートIP設定中..."
gcloud compute addresses create google-managed-services-default \
  --global \
  --purpose=VPC_PEERING \
  --prefix-length=16 \
  --network=default 2>/dev/null || echo "アドレスは既に存在します"

gcloud services vpc-peerings connect \
  --service=servicenetworking.googleapis.com \
  --ranges=google-managed-services-default \
  --network=default 2>/dev/null || echo "ピアリングは既に存在します"

echo "⏳ インスタンスをプライベートIPに更新中..."
gcloud sql instances patch $INSTANCE_NAME \
  --network=default \
  --no-assign-ip

# 接続情報の取得
echo "📡 接続情報を取得中..."
sleep 30  # インスタンスの更新を待つ

PRIVATE_IP=$(gcloud sql instances describe $INSTANCE_NAME \
  --format="value(ipAddresses[0].ipAddress)")
POSTGRES_URL="postgresql://${DB_USER}:${DB_PASSWORD}@${PRIVATE_IP}:5432"

echo ""
echo "=== セットアップ完了 ==="
echo "✅ インスタンス名: $INSTANCE_NAME"
echo "✅ プライベートIP: $PRIVATE_IP"
echo "✅ データベース名: $DB_NAME"
echo "✅ ユーザー名: $DB_USER"
echo "✅ パスワード: $DB_PASSWORD"
echo ""
echo "🔗 POSTGRES_URL: $POSTGRES_URL"
echo ""

# 設定情報をファイルに保存
cat > cloud-sql-config.txt << EOF
# Google Cloud SQL Configuration for Convex
INSTANCE_NAME=$INSTANCE_NAME
PRIVATE_IP=$PRIVATE_IP
DB_NAME=$DB_NAME
DB_USER=$DB_USER
DB_PASSWORD=$DB_PASSWORD
POSTGRES_URL=$POSTGRES_URL
EOF

echo "📄 設定情報を cloud-sql-config.txt に保存しました"
echo ""
echo "📋 次のステップ:"
echo "1. 上記の情報を安全な場所に保存"
echo "2. ./update-convex-database.sh を実行してConvexを更新"
echo ""

# Convex更新スクリプトの生成
cat > update-convex-database.sh << 'EOF'
#!/bin/bash
# Convex Database Update Script
set -e

if [ ! -f "cloud-sql-config.txt" ]; then
    echo "エラー: cloud-sql-config.txt が見つかりません"
    echo "先に cloud-sql-setup.sh を実行してください"
    exit 1
fi

# 設定読み込み
source cloud-sql-config.txt

echo "=== Convex Database Update ==="
echo "現在のデータをバックアップしています..."

# Convexサーバーでデータベース更新
gcloud compute ssh convex-backend-instance --zone=asia-northeast1-a --command="
set -e
cd /opt/convex

# 現在のデータをエクスポート
echo '💾 データをエクスポート中...'
sudo docker-compose exec -T backend npx convex export --path backup-before-cloudsql-\$(date +%Y%m%d).zip || echo 'データエクスポートに失敗しました（空のデータベースの可能性）'

# 環境変数ファイルの更新
echo '⚙️  環境変数を更新中...'
echo 'POSTGRES_URL=$POSTGRES_URL' | sudo tee -a .env

# サービスの再起動
echo '🔄 サービスを再起動中...'
sudo docker-compose down
sudo docker-compose up -d

# 接続確認
echo '🔍 接続を確認中...'
sleep 30
sudo docker-compose logs backend | grep -i postgres || echo 'PostgreSQL接続ログを確認してください'

echo '✅ データベース更新完了!'
"

echo ""
echo "=== 更新完了 ==="
echo "PostgreSQL接続が設定されました"
echo ""
echo "📊 ログを確認:"
echo "gcloud compute ssh convex-backend-instance --zone=asia-northeast1-a --command='sudo docker-compose logs backend | grep -i postgres'"
echo ""
echo "🔍 接続テスト:"
echo "gcloud compute ssh convex-backend-instance --zone=asia-northeast1-a --command='cd /opt/convex && sudo docker-compose exec -T backend psql \$POSTGRES_URL/convex_self_hosted -c \"SELECT version();\"'"
EOF

chmod +x update-convex-database.sh

echo "🚀 ./update-convex-database.sh スクリプトを作成しました"
echo "このスクリプトを実行してConvexをCloud SQLに接続してください"