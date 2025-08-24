# Convex GCP Hosting Deployment Scripts 使用方法

このディレクトリには、Google Cloud Platform上でConvexバックエンドをセルフホスティングするためのスクリプトとガイドが含まれています。

## 📁 ファイル構成

```
deployment/
├── CONVEX_GCP_HOSTING_GUIDE.md    # 詳細なセットアップガイド
├── quick-setup.sh                 # ワンクリックセットアップスクリプト
├── README_USAGE.md               # このファイル（使用方法）
├── contact-template.yaml         # Google Cloud Domains用連絡先テンプレート
├── dns-setup.sh                 # DNS設定スクリプト
├── gcp-setup.sh                 # GCPインフラ設定スクリプト
├── ssl-setup.sh                 # SSL証明書設定スクリプト
└── startup.sh                   # サーバー初期設定スクリプト
```

## 🚀 クイックスタート

### 方法1: ワンクリックセットアップ（推奨）

```bash
# 実行権限を付与
chmod +x deployment/quick-setup.sh

# セットアップ実行
./deployment/quick-setup.sh your-domain.com your-email@example.com
```

**例:**
```bash
./deployment/quick-setup.sh mycompany-convex.com admin@mycompany.com
```

このスクリプトは以下を自動実行します：
- GCPファイアウォールルール作成
- Compute Engineインスタンス作成
- 自動セットアップスクリプトの配置
- DNS設定用スクリプトの生成
- SSL証明書設定用スクリプトの生成

### 方法2: 手動セットアップ

詳細な手順については `CONVEX_GCP_HOSTING_GUIDE.md` を参照してください。

## 📋 セットアップ後の手順

ワンクリックセットアップ実行後、以下の手順を実行してください：

### 1. DNS設定

#### Google Cloud DNSを使用する場合
```bash
./setup-dns.sh
```

#### 外部ドメインレジストラーを使用する場合
スクリプト実行時に表示されたDNSレコードを手動で設定

### 2. SSL証明書設定（DNS反映後）

```bash
# DNS反映確認（通常24-48時間後）
nslookup your-domain.com
nslookup api.your-domain.com
nslookup dashboard.your-domain.com

# SSL証明書設定
./setup-ssl.sh
```

### 3. 管理キー取得

```bash
./get-admin-key.sh
```

### 4. ダッシュボードアクセス

1. https://dashboard.your-domain.com にアクセス
2. 取得した管理キーを入力
3. デプロイメントがオンラインになることを確認

## 🔧 個別スクリプトの使用方法

### ドメイン取得用連絡先設定
```bash
# contact-template.yamlを編集
vim deployment/contact-template.yaml

# Google Cloud Domainsでドメイン登録
gcloud domains registrations register your-domain.com \
  --contact-data-from-file=deployment/contact-template.yaml \
  --contact-privacy=redacted-contact-data \
  --yearly-price="12.00 USD"
```

### GCPインフラのみ作成
```bash
# プロジェクトIDを編集
vim deployment/gcp-setup.sh

# 実行
./deployment/gcp-setup.sh
```

### DNS設定のみ実行
```bash
# 設定を編集
vim deployment/dns-setup.sh

# 実行
./deployment/dns-setup.sh
```

### SSL証明書のみ設定
```bash
# メールアドレスを編集
vim deployment/ssl-setup.sh

# 実行（サーバー上で）
./ssl-setup.sh
```

## 🔍 トラブルシューティング

### よくある問題

#### 1. スクリプト実行権限エラー
```bash
chmod +x deployment/*.sh
```

#### 2. gcloud認証エラー
```bash
gcloud auth login
gcloud config set project YOUR_PROJECT_ID
```

#### 3. DNS設定の確認
```bash
# DNS propagation checker
nslookup your-domain.com
# または
dig your-domain.com
```

#### 4. サービス状態の確認
```bash
# インスタンスにSSH
gcloud compute ssh convex-backend-instance --zone=asia-northeast1-a

# サービス確認
sudo docker-compose ps
sudo docker-compose logs backend
```

### ログ確認方法

```bash
# インスタンス作成ログ
gcloud compute instances get-serial-port-output convex-backend-instance --zone=asia-northeast1-a

# Docker サービスログ
gcloud compute ssh convex-backend-instance --zone=asia-northeast1-a --command="sudo docker-compose logs -f"

# Nginxログ
gcloud compute ssh convex-backend-instance --zone=asia-northeast1-a --command="sudo tail -f /var/log/nginx/error.log"
```

## ⚙️ カスタマイズ

### マシンタイプの変更
```bash
# gcp-setup.sh または quick-setup.sh内で変更
MACHINE_TYPE="e2-standard-4"  # より高性能に
```

### リージョンの変更
```bash
# スクリプト内で変更
ZONE="us-central1-a"  # アメリカリージョン
```

### 追加ドメインの設定
```bash
# Nginx設定に追加
sudo vim /etc/nginx/sites-available/convex

# SSL証明書に追加
sudo certbot --nginx -d additional-domain.com
```

## 📊 運用・メンテナンス

### バックアップスクリプト作成例
```bash
#!/bin/bash
# backup.sh
DATE=$(date +%Y%m%d)
npx convex export --path "backup-${DATE}.zip"
npx convex env list > "env-backup-${DATE}.txt"
```

### モニタリングスクリプト作成例
```bash
#!/bin/bash
# monitor.sh
gcloud compute ssh convex-backend-instance --zone=asia-northeast1-a --command="
sudo docker-compose ps
sudo systemctl status nginx
df -h
free -m
"
```

## 📚 参考資料

- **詳細ガイド**: `CONVEX_GCP_HOSTING_GUIDE.md`
- **Convex公式ドキュメント**: https://docs.convex.dev/production/hosting/self-hosted
- **Google Cloud Console**: https://console.cloud.google.com/
- **Convex Dashboard**: https://dashboard.convex.dev/

## 🆘 サポート

問題が発生した場合：

1. `CONVEX_GCP_HOSTING_GUIDE.md` のトラブルシューティングセクションを確認
2. ログを確認して具体的なエラーメッセージを特定
3. Convex Discordの`#self-hosted`チャンネルで相談
4. GitHub Issuesで報告

---

**注意**: プロダクション環境では、セキュリティ設定、バックアップ戦略、モニタリングの追加実装を強く推奨します。