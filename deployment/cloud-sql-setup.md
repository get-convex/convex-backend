# Convex Self-Hosted with Google Cloud SQL (PostgreSQL) 設定ガイド

Google Cloud SQLを使用することで、Convexバックエンドをよりスケーラブルで高可用性な構成にできます。

## 🎯 利点

- **高可用性**: 自動フェイルオーバー
- **自動バックアップ**: ポイントインタイム リカバリ
- **スケーラビリティ**: リソース調整が容易
- **セキュリティ**: VPCネットワーク、暗号化対応
- **監視**: Cloud Monitoringとの統合

## 📋 前提条件

- Google Cloud Projectが作成済み
- gcloud CLIがインストール・認証済み
- Convexバックエンドが既にデプロイ済み（SQLiteで動作中）

## 🚀 セットアップ手順

### 1. Google Cloud SQL インスタンスの作成

```bash
# 環境変数の設定
export PROJECT_ID="your-gcp-project-id"
export REGION="asia-northeast1"  # 東京リージョン
export INSTANCE_NAME="convex-postgres"
export DB_NAME="convex_self_hosted"
export DB_USER="convex_user"
export DB_PASSWORD="$(openssl rand -base64 32)"  # ランダムパスワード生成

echo "データベースパスワード: $DB_PASSWORD"
echo "このパスワードを安全な場所に保存してください"

# Cloud SQL インスタンスの作成
gcloud sql instances create $INSTANCE_NAME \
  --database-version=POSTGRES_15 \
  --tier=db-custom-2-4096 \
  --region=$REGION \
  --storage-type=SSD \
  --storage-size=100GB \
  --storage-auto-increase \
  --backup-start-time=03:00 \
  --maintenance-window-day=SUN \
  --maintenance-window-hour=04 \
  --enable-bin-log \
  --deletion-protection
```

### 2. データベースとユーザーの作成

```bash
# データベースの作成
gcloud sql databases create $DB_NAME \
  --instance=$INSTANCE_NAME

# ユーザーの作成
gcloud sql users create $DB_USER \
  --instance=$INSTANCE_NAME \
  --password=$DB_PASSWORD
```

### 3. ネットワーク設定

#### Option A: プライベートIP（推奨）

```bash
# VPCピアリングの設定
gcloud compute addresses create google-managed-services-default \
  --global \
  --purpose=VPC_PEERING \
  --prefix-length=16 \
  --network=default

gcloud services vpc-peerings connect \
  --service=servicenetworking.googleapis.com \
  --ranges=google-managed-services-default \
  --network=default

# プライベートIPでインスタンスを更新
gcloud sql instances patch $INSTANCE_NAME \
  --network=default \
  --no-assign-ip
```

#### Option B: パブリックIP + 承認済みネットワーク

```bash
# Compute Engineインスタンスの外部IPを取得
COMPUTE_EXTERNAL_IP=$(gcloud compute instances describe convex-backend-instance \
  --zone=asia-northeast1-a --format="get(networkInterfaces[0].accessConfigs[0].natIP)")

# 承認済みネットワークに追加
gcloud sql instances patch $INSTANCE_NAME \
  --authorized-networks=$COMPUTE_EXTERNAL_IP/32
```

### 4. 接続文字列の取得

```bash
# 接続情報の取得
gcloud sql instances describe $INSTANCE_NAME

# プライベートIPの場合
PRIVATE_IP=$(gcloud sql instances describe $INSTANCE_NAME \
  --format="value(ipAddresses[0].ipAddress)")

# パブリックIPの場合
PUBLIC_IP=$(gcloud sql instances describe $INSTANCE_NAME \
  --format="value(ipAddresses[1].ipAddress)")

# 接続文字列の構築（プライベートIPを推奨）
POSTGRES_URL="postgresql://${DB_USER}:${DB_PASSWORD}@${PRIVATE_IP}:5432"

echo "POSTGRES_URL: $POSTGRES_URL"
```

### 5. Convexバックエンドの設定更新

```bash
# サーバーにSSH接続
gcloud compute ssh convex-backend-instance --zone=asia-northeast1-a

# 現在のデータをエクスポート（バックアップ）
cd /opt/convex
npx convex export --path backup-before-cloudsql-$(date +%Y%m%d).zip

# 環境変数ファイルの更新
sudo bash -c "cat >> .env << EOF

# Google Cloud SQL PostgreSQL
POSTGRES_URL=${POSTGRES_URL}
EOF"

# Docker Composeサービスの再起動
sudo docker-compose down
sudo docker-compose up -d

# ログで接続確認
sudo docker-compose logs backend | grep -i postgres
```

### 6. データベース接続の確認

```bash
# PostgreSQL接続テスト
sudo docker-compose exec backend bash -c "
psql '$POSTGRES_URL/convex_self_hosted' -c 'SELECT version();'
"

# Convexログでの確認
sudo docker-compose logs backend | grep "Connected to Postgres"
```

## 🔧 自動化スクリプト

### Cloud SQL セットアップスクリプト

```bash
#!/bin/bash
# cloud-sql-setup.sh
set -e

# パラメータ
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

read -p "設定を確認しました。続行しますか? (y/N): " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "セットアップをキャンセルしました。"
    exit 1
fi

# Cloud SQL API の有効化
echo "Cloud SQL API を有効化中..."
gcloud services enable sqladmin.googleapis.com

# インスタンスの作成
echo "Cloud SQL インスタンスを作成中..."
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
echo "データベースとユーザーを作成中..."
gcloud sql databases create $DB_NAME --instance=$INSTANCE_NAME
gcloud sql users create $DB_USER --instance=$INSTANCE_NAME --password=$DB_PASSWORD

# プライベートIP設定
echo "プライベートIP設定中..."
gcloud compute addresses create google-managed-services-default \
  --global \
  --purpose=VPC_PEERING \
  --prefix-length=16 \
  --network=default || echo "アドレスは既に存在します"

gcloud services vpc-peerings connect \
  --service=servicenetworking.googleapis.com \
  --ranges=google-managed-services-default \
  --network=default || echo "ピアリングは既に存在します"

gcloud sql instances patch $INSTANCE_NAME \
  --network=default \
  --no-assign-ip

# 接続情報の取得
PRIVATE_IP=$(gcloud sql instances describe $INSTANCE_NAME \
  --format="value(ipAddresses[0].ipAddress)")
POSTGRES_URL="postgresql://${DB_USER}:${DB_PASSWORD}@${PRIVATE_IP}:5432"

echo ""
echo "=== セットアップ完了 ==="
echo "インスタンス名: $INSTANCE_NAME"
echo "プライベートIP: $PRIVATE_IP"
echo "データベース名: $DB_NAME"
echo "ユーザー名: $DB_USER"
echo "パスワード: $DB_PASSWORD"
echo ""
echo "POSTGRES_URL: $POSTGRES_URL"
echo ""
echo "次のステップ:"
echo "1. 上記の情報を安全な場所に保存"
echo "2. ./update-convex-database.sh を実行してConvexを更新"

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

echo "設定情報を cloud-sql-config.txt に保存しました"
```

### Convex更新スクリプト

```bash
#!/bin/bash
# update-convex-database.sh
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
echo 'データをエクスポート中...'
sudo docker-compose exec -T backend npx convex export --path backup-before-cloudsql-\$(date +%Y%m%d).zip

# 環境変数ファイルの更新
echo 'POSTGRES_URL=$POSTGRES_URL' | sudo tee -a .env

# サービスの再起動
echo 'サービスを再起動中...'
sudo docker-compose down
sudo docker-compose up -d

# 接続確認
echo '接続を確認中...'
sleep 30
sudo docker-compose logs backend | grep -i postgres || echo 'PostgreSQL接続ログを確認してください'

echo 'データベース更新完了!'
"

echo ""
echo "=== 更新完了 ==="
echo "PostgreSQL接続が設定されました"
echo "ログを確認してください:"
echo "gcloud compute ssh convex-backend-instance --zone=asia-northeast1-a --command='sudo docker-compose logs backend | grep -i postgres'"
```

## 🔍 監視とメンテナンス

### パフォーマンス監視

```bash
# Cloud SQL インスタンスの監視
gcloud sql instances describe $INSTANCE_NAME

# 接続数の確認
gcloud sql instances describe $INSTANCE_NAME \
  --format="value(stats.connections)"

# ストレージ使用量の確認
gcloud sql instances describe $INSTANCE_NAME \
  --format="value(stats.dataUsed)"
```

### バックアップ管理

```bash
# 手動バックアップの作成
gcloud sql backups create \
  --instance=$INSTANCE_NAME \
  --description="Manual backup $(date +%Y%m%d)"

# バックアップ一覧の確認
gcloud sql backups list --instance=$INSTANCE_NAME

# ポイントインタイム リカバリ
gcloud sql instances clone $INSTANCE_NAME $INSTANCE_NAME-clone \
  --point-in-time='2024-01-15T10:00:00Z'
```

## 💰 コスト最適化

### 推奨設定

```bash
# 開発環境用（小規模）
--tier=db-f1-micro
--storage-size=20GB

# 本番環境用（中規模）
--tier=db-custom-2-4096
--storage-size=100GB

# 本番環境用（大規模）
--tier=db-custom-4-8192
--storage-size=500GB
```

### 自動スケーリング設定

```bash
# ストレージの自動拡張
gcloud sql instances patch $INSTANCE_NAME \
  --storage-auto-increase \
  --storage-auto-increase-limit=1000

# リードレプリカの作成（読み取り負荷分散）
gcloud sql instances create $INSTANCE_NAME-replica \
  --master-instance-name=$INSTANCE_NAME \
  --tier=db-custom-1-2048 \
  --region=$REGION
```

## ⚠️ 注意事項

1. **レイテンシ**: ConvexバックエンドとCloud SQLを同一リージョンに配置
2. **接続数**: 適切な接続プールサイズの設定
3. **セキュリティ**: プライベートIPの使用を推奨
4. **バックアップ**: 定期的なバックアップとテスト復元
5. **監視**: Cloud Monitoringでのアラート設定

## 🔄 SQLiteからの移行

```bash
# 1. 現在のデータをエクスポート
npx convex export --path sqlite-backup.zip

# 2. Cloud SQL設定
# （上記手順を実行）

# 3. データのインポート
npx convex import --replace-all sqlite-backup.zip
```

---

このガイドに従うことで、ConvexバックエンドをGoogle Cloud SQLで高可用性かつスケーラブルな構成で運用できます。