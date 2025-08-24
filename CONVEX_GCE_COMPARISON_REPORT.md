# Convex GCE環境 開発用vs本番用 徹底比較レポート

## 🎯 概要
開発用（`convex-backend-dev`）と本番用（`convex-backend-instance`）のConvex GCE環境を徹底比較しました。

---

## 1. 💻 GCEインスタンス基本情報

| 項目 | 開発用（convex-backend-dev） | 本番用（convex-backend-instance） |
|------|---------------------------|--------------------------------|
| **マシンタイプ** | `e2-standard-2` (2vCPU, 8GB RAM) | `e2-standard-4` (4vCPU, 16GB RAM) |
| **ディスク** | 50GB pd-standard | 50GB pd-ssd |
| **作成日** | 2025-08-03 | 2025-07-08 |
| **外部IP** | 35.243.120.253 | 34.84.108.222 |
| **内部IP** | 動的割り当て | 動的割り当て |
| **ゾーン** | asia-northeast1-a | asia-northeast1-a |

### 📊 重要な違い
- **本番用は開発用の2倍のリソース**（CPU・RAM）
- **本番用はSSDディスク**（性能重視）
- **開発用は標準ディスク**（コスト重視）

---

## 2. 🐳 Docker設定とコンテナ構成

### 開発用の特徴
```yaml
# シンプルな構成
services:
  cloudsql-proxy: gcr.io/cloud-sql-connectors/cloud-sql-proxy:2.17.1
  backend: ghcr.io/get-convex/convex-backend:latest  
  dashboard: ghcr.io/get-convex/convex-dashboard:latest
```

### 本番用の特徴  
```yaml
# 本格運用向け詳細設定
services:
  cloudsql-proxy:
    # 同じイメージ + ネットワーク設定
    networks: [convex-network]
    
  backend:
    # 追加設定多数
    stop_grace_period: 10s
    stop_signal: SIGINT
    extra_hosts: ["host.docker.internal:host-gateway"]
    healthcheck:
      test: curl -f http://localhost:3210/version
      interval: 5s
      start_period: 10s
    # AWS/S3設定完備
    # 26の環境変数対応
    
  dashboard:
    # ヘルスチェック依存設定
    depends_on:
      backend:
        condition: service_healthy
```

### 🔍 重要な違い

| 項目 | 開発用 | 本番用 |
|------|--------|--------|
| **設定複雑度** | シンプル | 詳細・本格的 |
| **ヘルスチェック** | ❌ なし | ✅ あり |
| **グレースフル停止** | ❌ なし | ✅ 10秒設定 |
| **ネットワーク** | デフォルト | 専用ネットワーク |
| **環境変数** | 基本的な6個 | 包括的な26個 |
| **AWS/S3対応** | ❌ なし | ✅ 完備 |

---

## 3. 🔧 環境変数・データベース接続設定

### 開発用設定
```bash
# 基本設定のみ
CONVEX_SITE_URL=https://convex.site
POSTGRES_URL=postgresql://convex_user:[password]@cloudsql-proxy:5432
DO_NOT_REQUIRE_SSL=1
```

### 本番用設定  
```bash
# 本番ドメイン
CONVEX_CLOUD_ORIGIN=https://api.jbci-convex-dev.com
CONVEX_SITE_ORIGIN=https://jbci-convex-dev.com
NEXT_PUBLIC_DEPLOYMENT_URL=https://api.jbci-convex-dev.com

# 同じPostgreSQLを使用
POSTGRES_URL=postgresql://convex_user:[password]@host.docker.internal:5432
```

### 🔍 重要な違い

| 項目 | 開発用 | 本番用 |
|------|--------|--------|
| **ドメイン** | デフォルト | 専用ドメイン設定 |
| **設定項目数** | 6項目 | 26項目 |
| **AWS設定** | ❌ | ✅ S3バケット等 |
| **ログレベル** | 基本 | 詳細設定可能 |
| **タイムアウト設定** | ❌ | ✅ |

---

## 4. 🌐 ネットワーク・セキュリティ・ファイアウォール

### ネットワークタグ
- **開発用**: `convex-server-dev`
- **本番用**: `convex-server`

### ファイアウォールルール

| ルール名 | 対象 | 許可ポート | 用途 |
|----------|------|------------|------|
| `allow-convex-ports-dev` | 開発用 | 3210,3211,6791,80,443,8080 | 開発環境用 |
| `allow-convex-ports` | 本番用 | 3210,3211,3212,6791,80,443 | 本番環境用 |

### 🔍 重要な違い
- **開発用はポート8080も開放**（開発ツール用）
- **本番用はポート3212も開放**（管理用？）
- **両方とも0.0.0.0/0からアクセス可能**（要注意）

---

## 5. 💾 ストレージ・バックアップ設定

### ディスク構成

| 項目 | 開発用 | 本番用 |
|------|--------|--------|
| **ディスクタイプ** | pd-standard | pd-ssd |
| **サイズ** | 50GB | 50GB |
| **使用量** | 6.8GB (15%) | 24GB (48%) |
| **データ保存** | ローカルディスク | Dockerボリューム |

### データベース
**共通**: Cloud SQL PostgreSQL (`convex-postgres`) を使用
- **リージョン**: asia-northeast1
- **IP**: 34.146.126.118
- **両環境から同じDBに接続**

### 🔍 重要な違い
- **本番用は高速SSDディスク使用**
- **本番用の使用量が多い**（24GB vs 6.8GB）
- **本番用はDocker管理ボリューム使用**
- **開発用はシンプルなローカルストレージ**

---

## 🚨 重要な課題・推奨事項

### 1. セキュリティ課題
❌ **両環境とも0.0.0.0/0からアクセス可能**
→ IP制限またはVPN経由のアクセスを推奨

### 2. データベース共有リスク
❌ **開発用と本番用が同じPostgreSQLを使用**
→ 開発用専用DBの作成を強く推奨

### 3. 本番環境の設定が充実
✅ **本番用は運用に適した詳細設定**
- ヘルスチェック
- グレースフル停止
- 包括的な環境変数

### 4. リソース配分
✅ **本番用は適切にスケールアップ**（2倍のリソース）

---

## 📋 まとめ

| 観点 | 開発用 | 本番用 |
|------|--------|--------|
| **目的** | 開発・テスト | 本番運用 |
| **複雑さ** | シンプル | 詳細・堅牢 |
| **パフォーマンス** | 標準 | 高性能 |
| **運用性** | 基本的 | 本格的 |
| **コスト** | 低 | 中程度 |

**最も重要な発見**: データベースが共有されているため、開発環境での作業が本番データに影響する可能性があります。これは即座に分離することを強く推奨します。