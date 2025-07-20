# マイルストーン1: イベントストア実装 - SOW（作業指示書）

**プロジェクト**: 家族用TODO共有アプリ  
**作成日**: 2025-07-13  
**対象期間**: マイルストーン1  

## 📋 プロジェクト現状分析

### 完了済み（マイルストーン0）
- ✅ プロジェクト初期設定完了
- ✅ バックエンド基盤構築（Rust Lambda）
- ✅ インフラ基盤構築（AWS SAM）
- ✅ CI/CDパイプライン構築（GitHub Actions）
- ✅ フロントエンド基盤構築（React + TypeScript + Biome）

### 現在の実装状況
- **バックエンド**: シンプルなHTTPハンドラーのみ（挨拶メッセージ返却）
- **インフラ**: Lambda + API Gateway の基本構成のみ
- **依存関係**: 基本的なHTTP処理ライブラリのみ

## 🎯 マイルストーン1の目標

**目的**: イベントソーシングアーキテクチャの基盤となるイベントストアを実装し、ULID識別子とDynamoDB統合を完成させる。

### 成功基準（DoD: Definition of Done）
1. ✅ DynamoDB統合（イベントストア・プロジェクションテーブル）
2. ✅ ULID実装
3. ✅ CommandHandlerでのイベント保存
4. ✅ DynamoDB Streams設定
5. ✅ 統合テストが通る
6. ✅ リント・コード品質チェックが通る

## 📝 実装計画

### タスク依存関係

| タスク | ブロッキング | 説明 |
| --- | --- | --- |
| タスク1 | ー | 基本ライブラリと ULID 実装 |
| タスク2 | タスク1 | DynamoDB スキーマとクライアントが ULID に依存 |
| タスク3a | タスク2 | Repository がテーブル定義に依存 |
| タスク3b | タスク3a | CommandHandler が Repository に依存 |
| タスク4 | タスク3b | 統合テストは API 実装完了後 |
| タスク5 | タスク4 | ドキュメントは実装完了後に更新 |

### タスク1: 依存関係追加とULID実装
ドメインモデルはエヴァンスのDDDに従い、各フィールドを値オブジェクトで型定義する。(ニュータイプイディオム)

#### 作業内容
1. **Cargo.toml更新**
   - `ulid = "1.1"`
   - `aws-sdk-dynamodb = "1.0"`
   - `chrono = { version = "0.4", features = ["serde"] }`

2. **ULID実装** (`backend/src/lib.rs` 新規作成)
   ```rust
   // TodoId構造体の実装
   // ULID生成・変換・タイムスタンプ抽出機能
   ```

3. **ドメインモデル定義** (`backend/src/domain/` 新規作成)
   - `Todo`構造体
   - `TodoEvent`構造体（Create/Update/Complete/Delete）

### タスク2: DynamoDB設計と実装

#### 作業内容
1. **SAMテンプレート更新** (`infra/template.yaml`)
   - EventStoreTable定義
   - ProjectionTable定義
   - DynamoDB Streams設定
   - IAMロール権限追加

2. **DynamoDBクライアント実装** (`backend/src/repository/`)
   - イベントストア操作
   - プロジェクション操作

### タスク3a: Repository実装 (DynamoDB CRUD)

#### 作業内容
1. **イベントストア CRUD 実装** (`backend/src/repository/event_store.rs`)
2. **プロジェクション CRUD 実装** (`backend/src/repository/projection.rs`)
3. **エラーハンドリング & リトライポリシー**
4. **ユニットテスト** (`repository` モジュール)

### タスク3b: CommandHandler実装

#### 作業内容
1. **HTTPルーティング実装**
   - POST `/todos` - TODO作成
   - PUT `/todos/{id}` - TODO更新
   - POST `/todos/{id}/complete` - TODO完了
   - DELETE `/todos/{id}` - TODO削除

2. **ビジネスロジック実装**
   - イベント生成
   - Repository 経由で DynamoDB 保存
   - レスポンス返却

### タスク4: 統合テスト実装

#### 作業内容
1. **テスト環境構築**
   - DynamoDB Local統合
   - テストデータ準備

2. **統合テスト実装**
   - API呼び出しテスト
   - DynamoDB操作確認
   - ULID生成確認

### タスク5: ドキュメント更新

#### 作業内容
1. **README.md更新**
   - ローカル開発手順
   - テスト実行方法

2. **CLAUDE.md更新**
   - 新しい実装コマンド追加

## 🗂️ ファイル構成（実装後）

```
backend/
├── src/
│   ├── main.rs              # エントリーポイント
│   ├── lib.rs               # ULID実装
│   ├── http_handler.rs      # HTTPハンドラー（更新）
│   ├── domain/              # ドメインモデル（新規）
│   │   ├── mod.rs
│   │   ├── todo.rs
│   │   └── events.rs
│   ├── repository/          # データ永続化（新規）
│   │   ├── mod.rs
│   │   ├── event_store.rs
│   │   └── projection.rs
│   └── handlers/            # ビジネスロジック（新規）
│       ├── mod.rs
│       └── command_handler.rs
├── tests/
│   ├── api_integration_tests.rs  # 統合テスト（更新）
│   └── event_store_tests.rs      # 新規テスト
└── Cargo.toml               # 依存関係（更新）
```

## 💾 DynamoDB テーブル設計

### EventStoreTable
| 属性 | 型 | キー | 説明 |
|------|----|----|------|
| PK | S | HASH | `FAMILY#{familyId}` |
| SK | S | RANGE | `EVENT#{ulid}` |
| EventType | S | - | `TodoCreated`, `TodoUpdated`, etc. |
| TodoId | S | - | ULID形式 |
| UserId | S | - | 実行者ID |
| Timestamp | S | - | ISO8601 |
| Data | M | - | イベント固有データ |

### ProjectionTable
| 属性 | 型 | キー | 説明 |
|------|----|----|------|
| PK | S | HASH | `FAMILY#{familyId}` |
| SK | S | RANGE | `TODO#{ulid}` |
| TodoId | S | - | ULID形式 |
| Title | S | - | TODOタイトル |
| Completed | BOOL | - | 完了フラグ |
| CreatedAt | S | - | 作成日時 |
| Version | N | - | 楽観的ロック用 |

## 🧪 テスト戦略
t-wadaのTDDに従い、テストファーストでRed-Green-Refactorを回す。

### ユニットテスト
- ULID生成・変換機能
- ドメインモデル検証
- ビジネスロジック

### 統合テスト
- HTTP API呼び出し
- DynamoDB操作
- エンドツーエンド処理

#### 受け入れ基準
- p95 レイテンシ < 200ms で全統合テストがパス
- エラー率 0% （テスト中に HTTP 5xx が発生しない）
- コードカバレッジ 80% 以上

### 実行コマンド
```bash
# ユニットテスト
cd backend && cargo test

# 統合テスト
cd backend && cargo test --test api_integration_tests

# リント
cd backend && cargo clippy -- -D warnings

# SAM ローカルテスト
cd infra && sam local start-api
```

## 📊 進捗管理
1つのテストにおける Red-Green-Refactor が完了するごとに、ユーザーに通知し、
承認を得るごとにGitコミットする

### 完了チェックリスト
- [ ] タスク1: 依存関係追加とULID実装
- [ ] タスク2: DynamoDB設計と実装
- [ ] タスク3a: Repository実装
- [ ] タスク3b: CommandHandler実装
- [ ] タスク4: 統合テスト実装
- [ ] タスク5: ドキュメント更新
- [ ] 全テスト通過確認
- [ ] リント・コード品質確認
- [ ] CI/CDパイプライン動作確認

### マイルストーン1完了後の状態
- イベントソーシングアーキテクチャの基盤完成
- ULID識別子による効率的なソート・範囲クエリ対応
- DynamoDB Streams設定済み（マイルストーン2のイベントプロセッサー準備完了）
- REST API基本機能（CRUD操作）実装完了
- 統合テスト環境構築完了

## 🔄 マイルストーン2への準備

マイルストーン1完了により、以下が準備完了となる：
- **イベントプロセッサー実装**: DynamoDB Streamsからのイベント処理
- **プロジェクション更新**: イベントからの読み取りモデル構築
- **QueryHandler実装**: 効率的な読み取りAPI

---

**🚀 次のアクション**: タスク1から順次実装開始