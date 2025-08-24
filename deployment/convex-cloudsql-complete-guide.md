# Convex Self-Hosted + Google Cloud SQL 完全実装ガイド

このガイドでは、Convex Self-HostedをGoogle Cloud SQL PostgreSQLと統合する3つのアプローチを詳しく解説します。

## 🎯 実装アプローチの比較

| アプローチ | 複雑さ | セキュリティ | 安定性 | 推奨度 |
|-----------|--------|------------|--------|--------|
| [A] Cloud SQL Auth Proxy | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| [B] SSL証明書 | ⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ |
| [C] SSL無効化 | ⭐ | ⭐ | ⭐⭐⭐ | ⭐⭐ |

## 📋 前提条件

- Google Cloud Project
- Convex Self-Hosted Backend (Docker)
- Google Compute Engine instance
- VPCネットワーク設定

## 🔄 アプローチ A: Cloud SQL Auth Proxy（推奨）

### 利点
- ✅ Google推奨のセキュアなアプローチ
- ✅ IAM認証とCredential自動ローテーション
- ✅ TLS終端の自動処理
- ✅ Convex側の設定がシンプル

### 実装手順

#### 1. OAuth スコープの設定

```bash
# Compute Engineインスタンス停止
gcloud compute instances stop convex-backend-instance --zone=asia-northeast1-a

# Cloud SQL OAuth スコープを追加
gcloud compute instances set-service-account convex-backend-instance \
  --zone=asia-northeast1-a \
  --service-account=COMPUTE_SERVICE_ACCOUNT \
  --scopes=https://www.googleapis.com/auth/devstorage.read_only,https://www.googleapis.com/auth/logging.write,https://www.googleapis.com/auth/monitoring.write,https://www.googleapis.com/auth/sqlservice

# インスタンス再開
gcloud compute instances start convex-backend-instance --zone=asia-northeast1-a
```

#### 2. Docker Compose設定

```yaml
# docker-compose-proxy.yml
services:
  cloudsql-proxy:
    image: gcr.io/cloud-sql-connectors/cloud-sql-proxy:2.17.1
    command:
      - "--address=0.0.0.0"
      - "--port=5432"
      - "--private-ip"
      - "PROJECT_ID:REGION:INSTANCE_NAME"
    ports:
      - "5432:5432"
    restart: unless-stopped

  backend:
    image: ghcr.io/get-convex/convex-backend:latest
    environment:
      - POSTGRES_URL=postgresql://USER:PASSWORD@cloudsql-proxy:5432
      - DO_NOT_REQUIRE_SSL=1
    depends_on:
      - cloudsql-proxy
```

#### 3. 環境変数設定

```bash
# .env
POSTGRES_URL=postgresql://convex_user:PASSWORD@cloudsql-proxy:5432
DO_NOT_REQUIRE_SSL=1
```

### トラブルシューティング

**問題**: `ACCESS_TOKEN_SCOPE_INSUFFICIENT`
```bash
# 解決策: OAuth スコープの確認と追加
gcloud compute instances describe INSTANCE_NAME --format="get(serviceAccounts[0].scopes[])"
```

**問題**: `Connection refused`
```bash
# 解決策: プロキシのアドレス設定確認
# --address=0.0.0.0 が必要（127.0.0.1ではコンテナ間通信不可）
```

## 🔐 アプローチ B: SSL証明書

### 利点
- ✅ 直接Cloud SQL接続
- ✅ 証明書による認証
- ✅ Docker Compose設定がシンプル

### 実装手順

#### 1. SSL証明書の取得

```bash
# Cloud SQL Server CA証明書のダウンロード
gcloud sql instances describe INSTANCE_NAME \
  --format='value(serverCaCert.cert)' > server-ca.pem
```

#### 2. Docker Compose設定

```yaml
# docker-compose-ssl.yml
services:
  backend:
    image: ghcr.io/get-convex/convex-backend:latest
    volumes:
      - ./server-ca.pem:/etc/ssl/certs/server-ca.pem:ro
    environment:
      - POSTGRES_URL=postgresql://USER:PASSWORD@PRIVATE_IP:5432
      - PGSSLMODE=verify-ca
      - PGSSLROOTCERT=/etc/ssl/certs/server-ca.pem
```

#### 3. 環境変数設定

```bash
# .env
POSTGRES_URL=postgresql://convex_user:PASSWORD@PRIVATE_IP:5432
PGSSLMODE=verify-ca
PGSSLROOTCERT=/etc/ssl/certs/server-ca.pem
```

### 必要な設定

#### VPCピアリング設定

```bash
# VPCアドレス範囲の作成
gcloud compute addresses create google-managed-services-default \
  --global \
  --purpose=VPC_PEERING \
  --prefix-length=16 \
  --network=default

# VPCピアリング接続
gcloud services vpc-peerings connect \
  --service=servicenetworking.googleapis.com \
  --ranges=google-managed-services-default \
  --network=default

# Cloud SQLインスタンスをプライベートIPに更新
gcloud sql instances patch INSTANCE_NAME \
  --network=default \
  --no-assign-ip
```

## ⚠️ アプローチ C: SSL無効化（非推奨）

### 注意点
- ❌ セキュリティリスクが高い
- ❌ 本番環境での使用は非推奨
- ✅ 開発・テスト環境でのみ使用

### 設定方法

```bash
# Cloud SQL SSL要求を無効化
gcloud sql instances patch INSTANCE_NAME --no-require-ssl
gcloud sql instances patch INSTANCE_NAME --ssl-mode=ALLOW_UNENCRYPTED_AND_ENCRYPTED

# Convex設定
POSTGRES_URL=postgresql://USER:PASSWORD@IP:5432
DO_NOT_REQUIRE_SSL=1
```

## 🛠️ 完全な実装スクリプト

### 1. 自動セットアップスクリプト

```bash
#!/bin/bash
# setup-convex-cloudsql.sh

# 変数設定
PROJECT_ID=$(gcloud config get-value project)
REGION="asia-northeast1"
INSTANCE_NAME="convex-postgres"
COMPUTE_INSTANCE="convex-backend-instance"

echo "=== Convex + Cloud SQL セットアップ ==="

# 1. VPCピアリング設定
echo "📡 VPCピアリング設定中..."
gcloud compute addresses create google-managed-services-default \
  --global --purpose=VPC_PEERING --prefix-length=16 --network=default

gcloud services vpc-peerings connect \
  --service=servicenetworking.googleapis.com \
  --ranges=google-managed-services-default --network=default

# 2. Cloud SQLインスタンス更新
echo "🗄️ Cloud SQLインスタンス更新中..."
gcloud sql instances patch $INSTANCE_NAME \
  --network=default --no-assign-ip

# 3. Compute Engineスコープ更新
echo "🔑 OAuth スコープ更新中..."
gcloud compute instances stop $COMPUTE_INSTANCE --zone=${REGION}-a
gcloud compute instances set-service-account $COMPUTE_INSTANCE \
  --zone=${REGION}-a \
  --scopes=https://www.googleapis.com/auth/sqlservice,https://www.googleapis.com/auth/devstorage.read_only
gcloud compute instances start $COMPUTE_INSTANCE --zone=${REGION}-a

echo "✅ セットアップ完了"
```

### 2. 設定ファイル生成

```bash
#!/bin/bash
# generate-config.sh

# Cloud SQL情報取得
PRIVATE_IP=$(gcloud sql instances describe convex-postgres \
  --format="value(ipAddresses[0].ipAddress)")
DB_PASSWORD=$(cat cloud-sql-config.txt | grep DB_PASSWORD | cut -d'=' -f2)

# Docker Compose生成（Proxy版）
cat > docker-compose.yml << EOF
services:
  cloudsql-proxy:
    image: gcr.io/cloud-sql-connectors/cloud-sql-proxy:2.17.1
    command:
      - "--address=0.0.0.0"
      - "--port=5432"
      - "--private-ip"
      - "${PROJECT_ID}:${REGION}:convex-postgres"
    ports:
      - "5432:5432"

  backend:
    image: ghcr.io/get-convex/convex-backend:latest
    environment:
      - POSTGRES_URL=postgresql://convex_user:${DB_PASSWORD}@cloudsql-proxy:5432
      - DO_NOT_REQUIRE_SSL=1
    depends_on:
      - cloudsql-proxy
EOF

# 環境変数生成
cat > .env << EOF
POSTGRES_URL=postgresql://convex_user:${DB_PASSWORD}@cloudsql-proxy:5432
DO_NOT_REQUIRE_SSL=1
EOF

echo "設定ファイルを生成しました"
```

## 🔍 診断とトラブルシューティング

### 接続診断スクリプト

```bash
#!/bin/bash
# diagnose-connection.sh

echo "=== Convex Cloud SQL 接続診断 ==="

# 1. OAuth スコープ確認
echo "1. OAuth スコープ確認:"
gcloud compute instances describe convex-backend-instance \
  --zone=asia-northeast1-a --format="get(serviceAccounts[0].scopes[])"

# 2. VPCピアリング確認
echo "2. VPCピアリング状態:"
gcloud services vpc-peerings list --network=default

# 3. Cloud SQL状態確認
echo "3. Cloud SQL状態:"
gcloud sql instances describe convex-postgres \
  --format="value(state,settings.ipConfiguration.privateNetwork)"

# 4. ネットワーク接続テスト
echo "4. ネットワーク接続テスト:"
PRIVATE_IP=$(gcloud sql instances describe convex-postgres \
  --format="value(ipAddresses[0].ipAddress)")
nc -z $PRIVATE_IP 5432 && echo "接続OK" || echo "接続NG"

# 5. PostgreSQL接続テスト
echo "5. PostgreSQL接続テスト:"
export PGPASSWORD='YOUR_PASSWORD'
timeout 5 psql "postgresql://convex_user@$PRIVATE_IP:5432/convex_self_hosted?sslmode=disable" \
  -c "SELECT version();" && echo "PostgreSQL接続OK" || echo "PostgreSQL接続NG"
```

### よくある問題と解決法

#### 1. Access Token Scope Insufficient

**症状**: Cloud SQL Auth Proxyで認証エラー
```
failed to get instance metadata: googleapi: Error 403: 
Request had insufficient authentication scopes.
```

**解決策**:
```bash
# OAuth スコープに sqlservice を追加
gcloud compute instances set-service-account INSTANCE_NAME \
  --scopes=https://www.googleapis.com/auth/sqlservice
```

#### 2. Connection Refused

**症状**: Convex backendがProxyに接続できない
```
Error: error connecting to server: Connection refused (os error 111)
```

**解決策**:
```bash
# Proxyの listen アドレスを修正
# --address=127.0.0.1 → --address=0.0.0.0
```

#### 3. VPC Peering Not Found

**症状**: プライベートIP接続ができない

**解決策**:
```bash
# VPCピアリングの再設定
gcloud services vpc-peerings connect \
  --service=servicenetworking.googleapis.com \
  --ranges=google-managed-services-default \
  --network=default
```

#### 4. Postgres Timeout

**症状**: PostgreSQL接続でタイムアウト

**解決策**:
```bash
# 接続タイムアウトの調整
POSTGRES_URL="postgresql://user:pass@host:5432?connect_timeout=30"

# または、プール設定の調整
POSTGRES_MAX_CONNECTIONS=20
```

## 📈 パフォーマンス最適化

### 接続プール設定

```bash
# .env
POSTGRES_MAX_CONNECTIONS=20
POSTGRES_IDLE_TIMEOUT=30
POSTGRES_ACQUIRE_TIMEOUT=60
```

### Cloud SQL最適化

```bash
# Cloud SQLフラグの設定
gcloud sql instances patch convex-postgres \
  --database-flags=max_connections=100 \
  --database-flags=shared_buffers=256MB \
  --database-flags=effective_cache_size=1GB
```

## 🔒 セキュリティベストプラクティス

### 1. IAM認証の使用

```bash
# IAMユーザーの作成
gcloud sql users create CONVEX_USER \
  --instance=convex-postgres \
  --type=cloud_iam_service_account \
  --project=PROJECT_ID
```

### 2. SSL証明書の定期更新

```bash
# 証明書の自動更新スクリプト
#!/bin/bash
# update-ssl-cert.sh
gcloud sql instances describe convex-postgres \
  --format='value(serverCaCert.cert)' > /opt/convex/server-ca.pem
docker-compose restart backend
```

### 3. 監視とアラート

```bash
# Cloud SQL メトリクス監視
gcloud sql instances describe convex-postgres \
  --format="value(stats.cpuUtilization,stats.memoryUtilization)"
```

## 🎯 推奨実装パス

### 開発環境
1. **SSL無効化**アプローチで概念実証
2. **SSL証明書**アプローチで基本機能確認
3. **Cloud SQL Auth Proxy**で本格実装

### 本番環境
1. **Cloud SQL Auth Proxy**アプローチのみ使用
2. IAM認証の有効化
3. 監視とアラートの設定
4. 定期的なセキュリティ監査

## 📚 関連リソース

- [Cloud SQL Auth Proxy Documentation](https://cloud.google.com/sql/docs/postgres/connect-auth-proxy)
- [Convex Self-Hosted Documentation](https://github.com/get-convex/convex-backend/tree/main/self-hosted)
- [VPC Peering Setup Guide](https://cloud.google.com/sql/docs/postgres/configure-private-ip)

---

この完全実装ガイドにより、Convex Self-HostedとGoogle Cloud SQLの統合が確実に実現できます。