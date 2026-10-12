# Rust Web App Template

Rust + Axum + PostgreSQL の最小 Web API テンプレート。設計方針は [Issue #1](https://github.com/tamura-kami/rust-web-app-template/issues/1) を参照。

## 必要環境

- Rust stable（最低 1.98、edition 2024）。`rust-toolchain.toml` で stable、rustfmt、clippy を指定。
- Docker / Docker Compose

[Axum 0.8.9](https://docs.rs/axum/0.8.9/axum/)、Tokio 1、[SQLx 0.9](https://docs.rs/sqlx/0.9.0/sqlx/)（Tokio + rustls + PostgreSQL）を採用。`Cargo.lock` を管理して依存解決を再現する。SQL はパラメータバインド付きの `query_as` で実行するため、コンパイル時の DB 接続や SQLx オフラインキャッシュは不要。SQL とスキーマの整合性は DB 統合テストで検証する。

## 起動

```sh
cp .env.example .env
# .env は信頼できるローカル設定として編集する。アプリは自動では読み込まない。
set -a
. ./.env
set +a
docker compose up -d --wait db
cargo run --locked
```

起動時に DB 接続と埋め込みマイグレーションを実行する。失敗した場合は HTTP の待受を開始せず終了する。SQLx が実行済みマイグレーションとチェックサムを管理する。既存 SQL は編集せず、新しい SQL ファイルを `migrations/` に追加する。実行バイナリの再ビルドで埋め込み内容を更新する。

```sh
curl http://127.0.0.1:3000/health
# {"status":"ok"}
curl http://127.0.0.1:3000/books/1
# {"id":1,"title":"Rust Web API sample"}
```

`Ctrl-C` / Unix の `SIGTERM` で graceful shutdown。DB の停止は `docker compose stop db`。データは名前付きボリュームに保持する。

## API

| メソッド | パス | 動作 |
| --- | --- | --- |
| GET | `/health` | プロセスの liveness。DB の readiness 検査ではない |
| GET | `/books/{id}` | 公開サンプル書籍の読み取り。正の i64 ID のみ許可 |

書籍 API は 200 で `{ "id": 1, "title": "..." }`、不正 ID は 400 / `invalid_book_id`、存在しない ID は 404 / `not_found`、内部障害は 500 / `internal_error`。エラー形式は `{ "error": "code" }`。内部原因は tracing に記録し、レスポンスに DB 情報を含めない。更新・削除・作成 API は用意しない。

入力は ID のみなのでハンドラで型と正数を検証する。JSON 入力が増えた段階で validator 等の導入を判断する。現時点の API 契約はこの README で管理し、OpenAPI / utoipa は API が増えるかクライアント生成が必要になった時点で導入する。

## 構成と依存方向

```text
root (起動 / ログ / 終了処理) → api / registry / shared
api (HTTP / DTO / API エラー) → registry
registry (具象 AppState / 依存構築) → adapter / kernel
adapter (SQLx / PostgreSQL / マイグレーション) → kernel
kernel (Book / BookRepository / RepositoryError)
shared (環境設定 / ConfigError)
```

`BookRepository` trait は kernel、`PgBookRepository` は adapter に置く。kernel は HTTP と SQLx に依存しない。registry は `Arc<dyn BookRepository>` を持つ具象 `AppState` を Axum に渡す。registry 自体の trait は不要と判断した。API のテストでは手書きの Repository テストダブルを注入するため mockall は不要。業務ロジックが増えた場合に Service / Usecase 層を追加する。

## 設定と運用

| 環境変数 | 必須 / 初期値 | 用途 |
| --- | --- | --- |
| `DATABASE_URL` | 必須 | PostgreSQL 接続先 |
| `BIND_ADDR` | `127.0.0.1:3000` | HTTP 待受アドレス |
| `CORS_ALLOWED_ORIGINS` | 空（許可 Origin なし） | カンマ区切りの http(s) Origin。例: `https://app.example.com` |
| `RUST_LOG` | `info` | tracing のフィルタ。HTTP ログを出す例: `info,tower_http=debug` |

CORS は明示した Origin のみ許可。`*`、パス、認証情報、クエリ付き URL は起動時に拒否する。CORS は認証や API アクセス制御ではない。HTTP メソッドは GET のみ許可し、Cookie credentials は有効にしない。

Compose のパスワードはローカル開発専用で、ポートは loopback のみ公開する。PostgreSQL の `log_statement=all` は設定しない。本番では秘密管理、DB 権限、TLS、接続数、ログ保存方針を環境に合わせる。現在は起動時マイグレーションを採用しているため、DB ユーザーには DDL 権限が必要。本番で権限を分離する場合はマイグレーションをデプロイ工程へ移す。

## テストと CI

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
# .env を読み込み、Compose の DB を起動してから実行
cargo test --workspace --locked -- --ignored
```

通常テストは DB 不要で、設定、HTTP 成功・不正入力・404、内部エラーの秘匿、CORS を検証する。DB テストは PostgreSQL が必要なため `ignore` で区別する。adapter テストは SQLx が一時 DB を作成・削除し、マイグレーションの再実行と Repository を検証する（接続ユーザーには CREATEDB 権限が必要）。API 統合テストは指定 DB にマイグレーションを適用し、HTTP → registry → adapter → PostgreSQL の読取を検証する。テストには開発専用 DB を使用する。

GitHub Actions で fmt / clippy / 単体テスト / PostgreSQL 統合テストを実行し、別ジョブで `cargo audit` を実行する。手元で脆弱性検査する場合は `cargo install cargo-audit --locked` の後に `cargo audit`。

## 採用しない機能と認証

初期テンプレートには独自のログイン・ログアウト、パスワード管理、トークン発行、認証 Extractor / Repository / trait、Redis、OpenTelemetry / Jaeger、ジョブ処理を含めない。

認証方式はアプリごとに選定する。セルフホスト OIDC の候補は authentik / Keycloak / Kanidm。マネージド認証の候補は Auth0 / Clerk / Amazon Cognito / Supabase Auth / Firebase Authentication。ブラウザ中心なら OIDC + バックエンド管理の Cookie セッション（BFF）も候補になる。標準プロトコルと保守されたライブラリを使い、認証とアプリ固有の認可を分ける。機密データ、作成・更新・削除 API を追加する際は、公開前に認証と認可を実装する。
