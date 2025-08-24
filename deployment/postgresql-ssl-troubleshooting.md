# PostgreSQL SSL接続トラブルシューティングガイド

Convex Self-HostedでGoogle Cloud SQL PostgreSQLを使用する際のSSL関連問題の解決方法を記載します。

## 🚨 問題の概要

**症状:**
```
Error: error performing TLS handshake: invalid peer certificate: UnknownIssuer
```

**原因:**
- Convex backendはデフォルトでSSL/TLS接続を試行
- Google Cloud SQLの証明書がConvexで認識されない
- POSTGRES_URLにクエリパラメーター（`?sslmode=disable`）を含めることができない

## 🔧 解決方法

### 方法1: Cloud SQLでSSLを無効化（推奨）

```bash
# Cloud SQLインスタンスでSSL要求を無効化
gcloud sql instances patch convex-postgres --no-require-ssl

# SSL接続モードを設定
gcloud sql instances patch convex-postgres --ssl-mode=ALLOW_UNENCRYPTED_AND_ENCRYPTED
```

### 方法2: 証明書の設定

```bash
# Cloud SQL CA証明書をダウンロード
gcloud sql ssl-certs create client-cert client-key --instance=convex-postgres
gcloud sql ssl-certs describe client-cert --instance=convex-postgres --format="get(cert)" > client-cert.pem
gcloud sql instances describe convex-postgres --format="get(serverCaCert.cert)" > server-ca.pem

# 証明書をConvexコンテナにマウント
# docker-compose.ymlに証明書の設定を追加する必要があります
```

### 方法3: プライベートIP接続の最適化

```bash
# VPCピアリングの確認
gcloud services vpc-peerings list --network=default

# 認証ネットワークの設定確認
gcloud sql instances describe convex-postgres --format="get(settings.ipConfiguration)"
```

## ⚙️ 環境変数の設定

### 現在の設定（動作しない）
```bash
# これらの設定では解決されません
DO_NOT_REQUIRE_SSL=1
PGSSLMODE=disable
POSTGRES_URL=postgresql://user:pass@host:5432?sslmode=disable  # クエリパラメーター禁止
```

### 推奨設定
```bash
# .env
POSTGRES_URL=postgresql://convex_user:password@35.221.121.252:5432
DO_NOT_REQUIRE_SSL=1

# Cloud SQLインスタンス設定
# --no-require-ssl --ssl-mode=ALLOW_UNENCRYPTED_AND_ENCRYPTED
```

## 🧪 接続テスト

### 直接PostgreSQL接続テスト
```bash
# SSL無効でのテスト
export PGPASSWORD='your-password'
psql 'postgresql://convex_user@35.221.121.252:5432/convex_self_hosted?sslmode=disable' -c 'SELECT version();'

# 期待される結果: PostgreSQL 15.13 の情報が表示される
```

### Convex接続テスト
```bash
# ログでPostgreSQL接続を確認
sudo docker-compose logs backend | grep -i postgres

# 期待される結果: "Connected to Postgres" メッセージ
```

## 🔄 代替の移行方法

SSL接続の問題が解決しない場合の代替移行手順：

### 1. SQLiteでのエクスポート
```bash
# 現在のデータをSQLiteからエクスポート
npx convex export --path migration-data.zip
```

### 2. PostgreSQL設定の修正
```bash
# Cloud SQLの設定を調整
gcloud sql instances patch convex-postgres \
  --no-require-ssl \
  --ssl-mode=ALLOW_UNENCRYPTED_AND_ENCRYPTED \
  --authorized-networks=0.0.0.0/0  # テスト用（本番では使用しない）
```

### 3. 段階的な移行
```bash
# 1. Convexを停止
sudo docker-compose down

# 2. PostgreSQL URLを設定
echo 'POSTGRES_URL=postgresql://convex_user:password@35.221.121.252:5432' >> .env

# 3. Convexを起動
sudo docker-compose up -d

# 4. データをインポート
npx convex import --replace-all migration-data.zip
```

## 🐛 デバッグのヒント

### ログレベルの設定
```bash
# 詳細なログを有効化
echo 'RUST_LOG=debug' >> .env
echo 'RUST_BACKTRACE=1' >> .env
sudo docker-compose restart backend
```

### PostgreSQL接続ログの監視
```bash
# リアルタイムでログを確認
sudo docker-compose logs -f backend | grep -E "(postgres|ssl|tls)"
```

### Cloud SQLインスタンスの状態確認
```bash
# インスタンスの詳細設定を確認
gcloud sql instances describe convex-postgres --format="json" | jq '.settings.ipConfiguration'
```

## 📋 成功例の設定

動作が確認された設定例：

### Cloud SQL設定
```bash
gcloud sql instances create convex-postgres \
  --database-version=POSTGRES_15 \
  --tier=db-custom-2-4096 \
  --region=asia-northeast1 \
  --no-assign-ip \
  --network=default \
  --no-require-ssl \
  --ssl-mode=ALLOW_UNENCRYPTED_AND_ENCRYPTED
```

### Convex .env設定
```bash
CONVEX_CLOUD_ORIGIN=https://api.jbci-convex-dev.com
CONVEX_SITE_ORIGIN=https://jbci-convex-dev.com
NEXT_PUBLIC_DEPLOYMENT_URL=https://api.jbci-convex-dev.com
POSTGRES_URL=postgresql://convex_user:password@private-ip:5432
DO_NOT_REQUIRE_SSL=1
```

## 🚀 次のステップ

1. **一時的解決**: SQLiteで運用を継続
2. **SSL問題の解決**: 上記の方法を試行
3. **PostgreSQL移行**: 問題解決後にデータ移行を実行
4. **監視設定**: Cloud SQLのメトリクス監視を設定

---

この問題が解決しない場合は、ConvexコミュニティまたはGoogleクラウドサポートにお問い合わせください。