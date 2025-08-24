# Convex SQLite → Google Cloud SQL 移行ガイド

既存のConvexセルフホスティング環境（SQLite）をGoogle Cloud SQL（PostgreSQL）に移行する完全ガイドです。

## 🎯 移行の利点

### Before (SQLite)
- ❌ 単一障害点
- ❌ 自動バックアップなし
- ❌ スケーラビリティ制限
- ❌ レプリケーション不可

### After (Google Cloud SQL)
- ✅ 高可用性・自動フェイルオーバー
- ✅ 自動バックアップ・ポイントインタイム リカバリ
- ✅ 水平・垂直スケーリング
- ✅ リードレプリカ対応
- ✅ Cloud Monitoring統合

## 📋 移行チェックリスト

### 事前準備
- [ ] 現在のConvexデプロイメントが正常動作中
- [ ] データの完全バックアップ作成
- [ ] 移行時のダウンタイム計画
- [ ] ロールバック計画の策定

### 移行実行
- [ ] Cloud SQLインスタンス作成
- [ ] ネットワーク設定
- [ ] データ移行
- [ ] 接続設定更新
- [ ] 動作確認

### 移行後
- [ ] パフォーマンス監視
- [ ] バックアップ設定確認
- [ ] 旧環境クリーンアップ

## 🚀 移行手順

### Step 1: 現在のデータバックアップ

```bash
# 現在のConvexサーバーでデータをエクスポート
gcloud compute ssh convex-backend-instance --zone=asia-northeast1-a --command="
cd /opt/convex
sudo docker-compose exec -T backend npx convex export --path migration-backup-\$(date +%Y%m%d-%H%M%S).zip
"

# バックアップファイルのローカルダウンロード
gcloud compute scp convex-backend-instance:/opt/convex/migration-backup-*.zip ./backup/ --zone=asia-northeast1-a
```

### Step 2: 環境変数の現在値保存

```bash
# 現在の環境変数を保存
gcloud compute ssh convex-backend-instance --zone=asia-northeast1-a --command="
cd /opt/convex
sudo cat .env > current-env-backup.txt
sudo docker-compose exec -T backend npx convex env list > current-convex-env.txt
"

# ローカルにダウンロード
gcloud compute scp convex-backend-instance:/opt/convex/current-env-backup.txt ./backup/ --zone=asia-northeast1-a
gcloud compute scp convex-backend-instance:/opt/convex/current-convex-env.txt ./backup/ --zone=asia-northeast1-a
```

### Step 3: Cloud SQLセットアップ

```bash
# Cloud SQLセットアップスクリプトの実行
./cloud-sql-setup.sh
```

### Step 4: メンテナンスモードの開始

```bash
# サービスを一時停止（データ整合性のため）
gcloud compute ssh convex-backend-instance --zone=asia-northeast1-a --command="
cd /opt/convex
sudo docker-compose down
"

# メンテナンスページの表示（オプション）
gcloud compute ssh convex-backend-instance --zone=asia-northeast1-a --command="
sudo systemctl stop nginx
echo '<h1>Maintenance in Progress</h1><p>We are upgrading our database. Please check back in 30 minutes.</p>' | sudo tee /var/www/html/index.html
sudo systemctl start nginx
"
```

### Step 5: 最終データバックアップ

```bash
# 最終的なデータバックアップ
gcloud compute ssh convex-backend-instance --zone=asia-northeast1-a --command="
cd /opt/convex
sudo docker-compose up -d backend
sleep 30
sudo docker-compose exec -T backend npx convex export --path final-backup-\$(date +%Y%m%d-%H%M%S).zip
sudo docker-compose down
"
```

### Step 6: データベース接続の更新

```bash
# Convexデータベース設定を更新
./update-convex-database.sh
```

### Step 7: データの移行

```bash
# Cloud SQL環境でのデータインポート
gcloud compute ssh convex-backend-instance --zone=asia-northeast1-a --command="
cd /opt/convex
# サービスの起動確認
sudo docker-compose ps

# データのインポート
sudo docker-compose exec -T backend npx convex import --replace-all final-backup-*.zip

# 環境変数の復元
sudo docker-compose exec -T backend npx convex env set --from-file current-convex-env.txt
"
```

### Step 8: 動作確認

```bash
# 基本的な動作確認
gcloud compute ssh convex-backend-instance --zone=asia-northeast1-a --command="
cd /opt/convex
# サービス状態確認
sudo docker-compose ps

# PostgreSQL接続確認
sudo docker-compose logs backend | grep -i postgres

# APIエンドポイントの確認
curl -I https://api.jbci-convex-dev.com/version
"

# ダッシュボードでの確認
echo "https://dashboard.jbci-convex-dev.com でデータとテーブルを確認してください"
```

### Step 9: パフォーマンステスト

```bash
# 簡単なパフォーマンステスト
gcloud compute ssh convex-backend-instance --zone=asia-northeast1-a --command="
cd /opt/convex
# クエリ実行時間の測定
time sudo docker-compose exec -T backend npx convex run myQuery
"
```

### Step 10: 本番運用再開

```bash
# Nginxの復元
gcloud compute ssh convex-backend-instance --zone=asia-northeast1-a --command="
sudo rm -f /var/www/html/index.html
sudo systemctl reload nginx
"

# 最終確認
curl -I https://api.jbci-convex-dev.com/version
curl -I https://dashboard.jbci-convex-dev.com
```

## 🔄 ロールバック手順

万が一問題が発生した場合のロールバック手順：

```bash
# 1. サービス停止
gcloud compute ssh convex-backend-instance --zone=asia-northeast1-a --command="
cd /opt/convex
sudo docker-compose down
"

# 2. 環境変数の復元
gcloud compute ssh convex-backend-instance --zone=asia-northeast1-a --command="
cd /opt/convex
sudo cp current-env-backup.txt .env
"

# 3. サービス再起動
gcloud compute ssh convex-backend-instance --zone=asia-northeast1-a --command="
cd /opt/convex
sudo docker-compose up -d
"

# 4. データの復元（必要に応じて）
gcloud compute ssh convex-backend-instance --zone=asia-northeast1-a --command="
cd /opt/convex
sudo docker-compose exec -T backend npx convex import --replace-all migration-backup-*.zip
"
```

## 📊 移行後の監視

### Cloud SQLメトリクス

```bash
# CPU使用率
gcloud sql instances describe convex-postgres --format="value(stats.cpuUtilization)"

# メモリ使用率
gcloud sql instances describe convex-postgres --format="value(stats.memoryUtilization)"

# ディスク使用量
gcloud sql instances describe convex-postgres --format="value(stats.dataUsed)"

# 接続数
gcloud sql instances describe convex-postgres --format="value(stats.connections)"
```

### Convexアプリケーションメトリクス

```bash
# レスポンス時間の確認
gcloud compute ssh convex-backend-instance --zone=asia-northeast1-a --command="
cd /opt/convex
sudo docker-compose logs backend | grep -E '(query|mutation)' | tail -20
"

# エラーログの確認
gcloud compute ssh convex-backend-instance --zone=asia-northeast1-a --command="
cd /opt/convex
sudo docker-compose logs backend | grep -i error | tail -10
"
```

## 🔧 最適化のヒント

### 接続プールの設定

```bash
# .envファイルに接続プール設定を追加
echo "
# PostgreSQL接続プール設定
POSTGRES_MAX_CONNECTIONS=20
POSTGRES_IDLE_TIMEOUT=30
" >> .env
```

### パフォーマンス チューニング

```bash
# Cloud SQLインスタンスのパフォーマンス調整
gcloud sql instances patch convex-postgres \
  --database-flags=shared_preload_libraries=pg_stat_statements \
  --database-flags=max_connections=100 \
  --database-flags=shared_buffers=256MB
```

### 自動スケーリング設定

```bash
# ストレージ自動拡張の設定
gcloud sql instances patch convex-postgres \
  --storage-auto-increase \
  --storage-auto-increase-limit=1000GB
```

## 🚨 トラブルシューティング

### よくある問題

#### 1. 接続エラー

```bash
# 問題: "connection refused"
# 解決: VPCピアリングの確認
gcloud services vpc-peerings list --network=default

# 問題: "authentication failed"
# 解決: パスワードの確認
source cloud-sql-config.txt
echo $DB_PASSWORD
```

#### 2. パフォーマンス問題

```bash
# 問題: 遅いクエリ
# 解決: スロークエリログの確認
gcloud sql instances patch convex-postgres \
  --database-flags=log_min_duration_statement=1000
```

#### 3. データ不整合

```bash
# 問題: データが不完全
# 解決: 再インポート
cd /opt/convex
sudo docker-compose exec -T backend npx convex import --replace-all final-backup-*.zip
```

## 💰 コスト最適化

### 開発/テスト環境用設定

```bash
# 小規模インスタンス
gcloud sql instances patch convex-postgres \
  --tier=db-f1-micro \
  --storage-size=20GB
```

### 本番環境用設定

```bash
# 高可用性設定
gcloud sql instances patch convex-postgres \
  --availability-type=REGIONAL \
  --backup-location=asia-northeast1
```

## 📝 移行後チェックリスト

- [ ] 全ての機能が正常動作
- [ ] パフォーマンスが許容範囲内
- [ ] バックアップが正常動作
- [ ] 監視アラートが設定済み
- [ ] 旧SQLiteファイルの削除
- [ ] ドキュメントの更新

---

この移行ガイドに従うことで、SQLiteからGoogle Cloud SQLへの安全な移行が可能です。問題が発生した場合は、すぐにロールバック手順を実行してください。