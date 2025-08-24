# Convex Backend GCP Deployment Guide

## 概要
このガイドでは、ConvexバックエンドをGoogle Cloud Platform（GCP）にデプロイし、独自ドメイン（jbci-convex-dev.com）で運用する手順を説明します。

## 前提条件
- GCPアカウントとプロジェクトが作成済み
- gcloud CLIがインストール・認証済み
- ドメインの管理権限

## 手順

### 1. ドメインの取得

#### オプション1: Google Cloud Domains
```bash
# ドメインの検索
gcloud domains registrations search-domains --query="jbci-convex-dev.com"

# ドメインの登録（要連絡先情報設定）
gcloud domains registrations register jbci-convex-dev.com \
  --registrant-contact-from-file=contact.yaml \
  --yearly-price=12.00 \
  --location=global
```

#### オプション2: 他のドメインレジストラー
- お名前.com、ムームードメイン、Squarespace Domainsなどでドメインを取得
- 後でネームサーバーをGoogle Cloud DNSに設定

### 2. GCPインフラストラクチャの作成

```bash
# プロジェクトIDを設定
vim deployment/gcp-setup.sh  # PROJECT_IDを実際の値に変更

# 実行権限付与
chmod +x deployment/*.sh

# GCPインスタンスの作成
./deployment/gcp-setup.sh
```

実行後、以下の情報が表示されます：
- インスタンスの外部IPアドレス
- 設定すべきDNSレコード

### 3. DNS設定

#### Google Cloud DNSを使用する場合
```bash
# DNS設定スクリプトの編集
vim deployment/dns-setup.sh  # PROJECT_IDとSERVER_IPを設定

# DNS設定の実行
./deployment/dns-setup.sh
```

#### 外部ドメインレジストラーを使用する場合
以下のDNSレコードを手動で設定：

| タイプ | 名前 | 値 |
|--------|------|-----|
| A | jbci-convex-dev.com | [サーバーIP] |
| A | api.jbci-convex-dev.com | [サーバーIP] |
| A | dashboard.jbci-convex-dev.com | [サーバーIP] |

### 4. SSL証明書の設定

DNS設定が反映された後（通常24-48時間）：

```bash
# サーバーにSSH接続
gcloud compute ssh convex-backend-instance --zone=asia-northeast1-a

# SSL設定スクリプトの実行
sudo vim /opt/convex/ssl-setup.sh  # メールアドレスを設定
sudo chmod +x /opt/convex/ssl-setup.sh
sudo ./ssl-setup.sh
```

### 5. 管理キーの取得

```bash
# サーバー上で管理キーを確認
sudo cat /opt/convex/admin_key.txt
```

### 6. ローカル開発環境の設定

Convexプロジェクトに以下を`.env.local`に追加：

```bash
CONVEX_SELF_HOSTED_URL='https://api.jbci-convex-dev.com'
CONVEX_SELF_HOSTED_ADMIN_KEY='[管理キー]'
```

## アクセス先

セットアップ完了後：
- **API**: https://api.jbci-convex-dev.com
- **HTTP Actions**: https://jbci-convex-dev.com
- **Dashboard**: https://dashboard.jbci-convex-dev.com

## トラブルシューティング

### ログの確認
```bash
# インスタンスのシリアルポート出力
gcloud compute instances get-serial-port-output convex-backend-instance --zone=asia-northeast1-a

# サーバー上のDockerログ
sudo docker-compose logs -f
```

### DNSの確認
```bash
# DNS解決の確認
nslookup jbci-convex-dev.com
nslookup api.jbci-convex-dev.com
nslookup dashboard.jbci-convex-dev.com
```

### サービス状態の確認
```bash
# Dockerサービスの確認
sudo docker-compose ps

# Nginxの確認
sudo systemctl status nginx
```

## セキュリティ注意事項

1. **管理キーの保護**: admin_key.txtは機密情報です
2. **ファイアウォール**: 必要なポートのみ開放
3. **SSL証明書**: 自動更新の確認
4. **バックアップ**: 定期的なデータバックアップの実施

## メンテナンス

### バックアップ
```bash
# データのエクスポート
npx convex export --path backup-$(date +%Y%m%d).zip
```

### アップデート
```bash
# Dockerイメージの更新
sudo docker-compose pull
sudo docker-compose up -d
```