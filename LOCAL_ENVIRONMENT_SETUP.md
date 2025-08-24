# 🚀 Convex Local環境セットアップ状況

## 📋 現在の構成

### ✅ 稼働中のlocal環境

**GCEインスタンス:**
- **名前**: `convex-backend-local`
- **外部IP**: `34.84.131.7`
- **内部IP**: `10.146.0.10`
- **ゾーン**: `asia-northeast1-a`
- **マシンタイプ**: `e2-standard-2`
- **ステータス**: ✅ 稼働中

**現在のデータベース接続 (一時的):**
- **Cloud SQLインスタンス**: `convex-postgres-dev` (dev環境と共有)
- **データベース名**: `convex_local`
- **ユーザー**: `convex_local_user`
- **パスワード**: `n4qZV9CnVNBDfk912psd0FTMQ`

**Admin Key:**
```
convex-local|0156191a0601e5ecd6bb225463893bce878beb6db1e0923ad871b3f1bfeedbb030a38ad985
```

**アクセスURL:**
- **Backend API**: `http://34.84.131.7:3210`
- **Dashboard**: `http://34.84.131.7:6791`
- **Site Proxy**: `http://34.84.131.7:3211`

---

## 🔄 専用Cloud SQL作成状況

**新規インスタンス情報:**
- **名前**: `convex-postgres-local-mini`
- **スペック**: db-f1-micro (最小構成)
  - CPU: 1 vCPU
  - メモリ: 614MB
  - ストレージ: 10GB HDD
  - 最大接続数: 25
- **ステータス**: ⏳ 作成中 (PENDING_CREATE)
- **作成開始時刻**: 2025-08-06 08:40:53 UTC

### 最小スペック設定の詳細:
- **ストレージタイプ**: HDD (コスト削減)
- **バックアップ**: 無効 (local環境のため)
- **自動ストレージ増加**: 無効
- **可用性**: ゾーナル (単一ゾーン)
- **データベースフラグ**:
  - `shared_buffers=16384` (メモリ最適化)
  - `max_connections=25` (接続数制限)

---

## 📝 切り替え手順 (インスタンス作成完了後)

1. **作成完了の確認**
```bash
gcloud sql instances describe convex-postgres-local-mini --format="get(state)"
# "RUNNABLE" が返されれば完了
```

2. **データベースとユーザーの作成**
```bash
# パスワード生成
PASSWORD=$(openssl rand -base64 32 | tr -d "=+/" | cut -c1-25)

# データベース作成
gcloud sql databases create convex_local --instance=convex-postgres-local-mini
gcloud sql databases create convex_self_hosted --instance=convex-postgres-local-mini

# ユーザー作成
gcloud sql users create convex_local_user --instance=convex-postgres-local-mini --password=$PASSWORD

echo "Password: $PASSWORD"
```

3. **Docker Compose更新**
```yaml
services:
  cloudsql-proxy-local:
    command:
      - "--address=0.0.0.0"
      - "--port=5432"
      - "--private-ip"
      - "ai-sales-hub-dev-new:asia-northeast1:convex-postgres-local-mini"  # ← ここを変更
```

4. **サービス再起動**
```bash
gcloud compute ssh convex-backend-local --zone=asia-northeast1-a
docker-compose down
docker-compose up -d
```

---

## 🛠️ 管理コマンド

### サービス状態確認
```bash
gcloud compute ssh convex-backend-local --zone=asia-northeast1-a --command="docker-compose ps"
```

### ログ確認
```bash
gcloud compute ssh convex-backend-local --zone=asia-northeast1-a --command="docker-compose logs backend"
```

### 再起動
```bash
gcloud compute ssh convex-backend-local --zone=asia-northeast1-a --command="docker-compose restart"
```

### Admin Key再生成
```bash
gcloud compute ssh convex-backend-local --zone=asia-northeast1-a --command="docker exec \$(docker ps -q --filter ancestor=ghcr.io/get-convex/convex-backend:latest) /convex/generate_admin_key.sh"
```

---

## 💰 コスト比較

### 現在 (dev環境と共有)
- GCE: ~$50/月
- Cloud SQL: $0 (dev環境と共有)
- **合計**: ~$50/月

### 専用Cloud SQL作成後
- GCE: ~$50/月
- Cloud SQL (db-f1-micro): ~$10/月
- **合計**: ~$60/月

---

## 📊 環境比較

| 項目 | Production | Development | Local |
|------|-----------|------------|--------|
| **GCE外部IP** | (未確認) | 35.243.120.253 | 34.84.131.7 |
| **Cloud SQL** | convex-postgres | convex-postgres-dev | convex-postgres-local-mini (作成中) |
| **データベース** | convex_self_hosted | convex_dev | convex_local |
| **マシンタイプ** | e2-standard-2 | e2-standard-2 | e2-standard-2 |
| **Cloud SQLスペック** | db-f1-micro | db-f1-micro | db-f1-micro |

---

## ⚠️ 注意事項

1. **専用Cloud SQL作成中**: 現在作成処理中のため、一時的にdev環境のCloud SQLを使用
2. **データ分離**: データベースレベルで分離されているため、データの混在はなし
3. **セキュリティ**: ファイアウォールルールで管理IPのみアクセス可能

---

最終更新: 2025-08-06 17:50 JST