# ToDoアプリ — AWSサーバーレス設計

**版:** 2025-08-30  
**目的:** イベントソーシング＋CQRS を学びつつ、少人数で安全に運用できるサーバーレス ToDo を構築する。

---

## 0. スコープ
- シングルプロダクト（家族=テナント）向け ToDo 管理
- 認証: Cognito（Passkey/Hosted UI, OAuth2 Auth Code + PKCE）
- 期間: PoC → 本番最小リリース（stg → prod）

---

## 1. アーキテクチャ概要
- **UI/SPA**: S3 + CloudFront（OAC 経由、S3 は完全非公開）
- **API**: API Gateway HTTP API + Lambda（Rust, axum + lambda_web）
- **永続化**: DynamoDB（イベントストア + プロジェクション）
- **非同期処理**: DynamoDB Streams → Lambda EventProcessor（SQS DLQ 付き）
- **監視**: CloudWatch（メトリクス/ログ/アラーム）, X-Ray（トレーシング）
- **IaC/CI**: AWS SAM, GitHub Actions（OIDC Assume Role）

---

## 2. ドメイン/イベント
- **集約 (Aggregate)**: `Todo`（1つの Todo が順序保証の単位）
- **イベント種類（例）**
  - `TodoCreated { title }`
  - `TodoTitleChanged { title }`
  - `TodoCompleted {}` / `TodoReopened {}`
  - `TodoDeleted {}`
- **メタ**: `eventId(=ULID), aggregateVersion, correlationId, causationId, actor(userId, familyId), timestamp`

---

## 3. データ設計

### 3.1 イベントストア（EventsTable）
- **キー設計（推奨: 集約単位のパーティション）**
  - `PK = TODO#${todoId}`
  - `SK = EVENT#${ulid}`
- **GSI（家族単位での集計/リプレイ用）**
  - `GSI1PK = FAMILY#${familyId}`
  - `GSI1SK = ${ulid}`
- **代表スキーマ**
```json
{
  "PK": "TODO#01HARX...",
  "SK": "EVENT#01HARX...",
  "GSI1PK": "FAMILY#f-123",
  "GSI1SK": "01HARX...",
  "aggregateType": "Todo",
  "aggregateId": "01HARX...",
  "aggregateVersion": 3,
  "eventId": "01HARX...",
  "eventType": "TodoCompleted",
  "data": { "completed": true },
  "metadata": {
    "correlationId": "c-...",
    "causationId": "k-...",
    "actor": { "userId": "u-1", "familyId": "f-123" },
    "timestamp": "2025-08-30T12:34:56.000Z"
  }
}
```

### 3.2 プロジェクション（TodosProjection）
- **キー**
  - `PK = FAMILY#${familyId}`
  - `SK = TODO#${todoId}`
- **GSI（未完了のみの一覧用）**
  - `GSI1PK = FAMILY#${familyId}#ACTIVE`（未完了の時だけ **属性を持つ**）
  - `GSI1SK = ${updatedAt}`（または `ulid`）
- **代表スキーマ**
```json
{
  "PK": "FAMILY#f-123",
  "SK": "TODO#01HARX...",
  "title": "Buy milk",
  "completed": false,
  "GSI1PK": "FAMILY#f-123#ACTIVE",
  "GSI1SK": "2025-08-30T12:34:56.000Z",
  "lastEventId": "01HARX...",
  "version": 3,
  "updatedAt": "2025-08-30T12:34:56.000Z"
}
```
- **注意**: `completed = true` になったら **更新で `REMOVE GSI1PK, GSI1SK`**（フィルタ式では削除されない）。

### 3.3 アイテムサイズとインデックス
- イベントは細粒度（1変更=1イベント）。`data` は必要最小限。
- 長文メモ等は別テーブル/オブジェクト格納を検討。

---

## 4. API 設計（要点）
- **認可境界**: `familyId` は **トークン(JWT)由来**。リクエストで受け取らない。
- **エンドポイント**
  - `POST /todos`（Idempotency-Key 対応）
  - `GET /todos?cursor=<lek>`（未完了のみ=GSI1, 完了一覧は別クエリ or クエリパラメータ）
  - `GET /todos/{id}`
  - `PATCH /todos/{id}`（タイトル変更/完了/再開など）
  - `DELETE /todos/{id}`
  - `GET /todos/{id}/history`（家族境界内のみ）
- **ページング**: `LastEvaluatedKey` ベースのカーソル方式。`nextCursor` をレスポンスに同梱。
- **Idempotency**
  - クライアント: `Idempotency-Key` ヘッダ（UUID）
  - サーバ: キー→`eventId` のマッピングを TTL 付きで保存
- **楽観ロック（競合制御）**
  - コマンド適用時: 期待 `aggregateVersion` を `ConditionExpression` で検証
  - 例（擬似）:
```
ConditionExpression: attribute_not_exists(aggregateVersion) OR aggregateVersion = :expected
UpdateExpression: SET aggregateVersion = :next, lastEventId = :eventId
```

---

## 5. イベント処理/再生（リビルド）
- **順序/整合性**
  - DynamoDB Streams の順序保証は **同一パーティションキー内** → 集約PK設計で要件を満たす
  - **冪等化**: プロジェクションは `lastEventId` を保持し、既処理 `eventId` は無視
- **障害対策**
  - EventProcessor に **SQS DLQ**。失敗イベントは隔離→再試行手順を Runbook に明記
- **リプレイ（全再生）**
  - 専用 Lambda/バッチ/Step Functions で、全 Todo のイベントを SK 順に再適用
  - 手順: Projection 全削除 → リプレイ → 整合性検証（件数/ハッシュ）

---

## 6. 認証/セキュリティ
- **Cognito + Hosted UI + PKCE**（SPA は **HttpOnly/SameSite** Cookie にトークンを格納）
- **Passkey (WebAuthn)**
  - RP ID = CloudFront カスタムドメイン（HTTPS 必須）
  - 同一サイト属性/サブドメイン戦略を整理
- **API Gateway JWT オーソライザー**
  - `familyId` / `memberOf` をカスタムクレームとして付与（Pre Token Generation で拡張可）
- **S3/CloudFront**
  - OAC を使用、S3 は **Block Public Access**、バケットポリシーは OAC のみ許可
- **IAM 最小権限**
  - テーブル/インデックス/操作を ARN で限定（例: `dynamodb:PutItem` でも対象テーブルを絞る）
- **CORS**
  - オリジンを CloudFront ドメインのみに限定。許可メソッド/ヘッダ/資格情報を最小化
- **ログ/PII**
  - 構造化 JSON ログ + `correlationId`/`causationId`
  - PII は最小化し、マスキング方針を決める

---

## 7. 監視/運用
- **メトリクス/アラーム**
  - API Gateway: 5xx, Latency (p95)
  - Lambda: Error %, Throttles, ConcurrentExecutions
  - DynamoDB: `ThrottledRequests`, `ConsumedRead/WriteCapacityUnits`
  - Streams: `GetRecords.IteratorAge.High`
  - SQS DLQ: `ApproximateAgeOfOldestMessage`
- **トレース**: X-Ray（API → Lambda → Dynamo の相関）
- **デプロイ後のヘルスチェック**: 合成監視（Canary）

---

## 8. コスト指針（ap-northeast-1 前提）
> ※概算式を示し、無料枠依存と満了後の費用を分けて評価。

| コンポーネント | 前提/式 | メモ |
|---|---|---|
| API Gateway | `リクエスト数 × 単価` | HTTP API |
| Lambda | `呼出数 × 単価 + GB-秒` | コールドスタート最適化 |
| DynamoDB | On-Demand `RCU/WCU` 消費 | テーブル/インデックス別に |
| CloudFront | `転送量 + リクエスト` | OAC, キャッシュ最適化 |
| S3 | `PUT/GET + ストレージ` | バケット非公開 |
| Cognito | `MAU` ベース | Passkey 追加コストなし |

---

## 9. テスト戦略
- **ドメインテスト**: コマンド→イベント写像（プロパティベース: `proptest` 推奨）
- **プロセッサテスト**: ストリーム入力→期待プロジェクション（冪等性/順序）
- **E2E**: 最終的整合を考慮したリトライ/待機を含む
- **負荷**: `k6` で p95 < 200ms を目標（API Gateway + Lambda + Dynamo 実測）
- **ステージング**: LocalStack では再現しづらい Cognito 周りは stg で検証

---

## 10. CI/CD・IaC
- **環境分離**: `dev/stg/prod`（アカウント/リージョン/スタック名分離）
- **GitHub→AWS**: OIDC + 条件付き権限（branch/repo 限定）
- **フロント配信**: コンテンツハッシュ付与、CloudFront は静的資産を長期、`index.html` のみ短期
- **データ移行/リプレイ**: IaC に運用関数（replay）と Runbook を含める

---

## 11. 非機能/SLO（初期）
- 可用性: 99.9%（月間）
- レイテンシ: API p95 < 200ms（読み取り系）, p95 < 350ms（書込み系）
- 一貫性: 最終的整合（プロジェクション反映 ≤ 数秒、SLO: p99 ≤ 10秒）

---

## 12. リスクと対応
- **整合性の遅延**: UI で「同期中」表示とリトライ設計
- **同時更新競合**: 楽観ロックとリトライポリシー
- **プロジェクション破損**: リプレイ手順/検証ハッシュ
- **権限漏れ**: IAM スコープの定期棚卸し

---

## 13. 次アクション
1. **イベントストアのキー/GSI 方針を確定**（集約PK + 家族GSI）
2. **EventProcessor の冪等/DLQ/リプレイ設計を IaC/Runbook に反映**
3. **JWT クレームに `familyId` を付与**し、API 認可をトークン起点に統一
4. **Projection 更新での GSI 属性付与/除去**を実装
5. **S3/CloudFront の OAC 化**（S3 完全非公開）
6. **テスト章（ドメイン/プロセッサ/E2E/負荷）**の雛形を追加し自動化
7. **コスト試算の前提/シナリオ表**を埋める（無料枠満了後も）

---

## 付録A: イベント適用の擬似コード（Rust）
```rust
enum Event { Created{title:String}, TitleChanged{title:String}, Completed, Reopened, Deleted }

struct Todo { title:String, completed:bool, version:u64 }

impl Todo {
  fn apply(&mut self, ev: &Event) {
    match ev {
      Event::Created{title} => { self.title = title.clone(); self.completed = false; },
      Event::TitleChanged{title} => { self.title = title.clone(); },
      Event::Completed => { self.completed = true; },
      Event::Reopened => { self.completed = false; },
      Event::Deleted => { /* tombstone フラグ等 */ },
    }
    self.version += 1;
  }
}
```

## 付録B: Projection 更新（完了→未完了）の疑似
```sql
-- 完了時: 未完了GSIを外す
UPDATE TodosProjection
SET completed = true, updatedAt = :now REMOVE GSI1PK, GSI1SK
WHERE PK = :family AND SK = :todo AND lastEventId < :eventId;

-- 再開時: 未完了GSIを付与
UPDATE TodosProjection
SET completed = false, updatedAt = :now, GSI1PK = :familyActive, GSI1SK = :now
WHERE PK = :family AND SK = :todo AND lastEventId < :eventId;
```

## 付録C: CloudFront/S3（要点）
- S3: Block Public Access = ON、ポリシーは OAC のみ許可
- CloudFront: OAC アタッチ、`/index.html` だけ短期キャッシュ、他は長期

## 付録D: 監視アラーム例
- `API 5xx > 1% (5m)`
- `Lambda Error% > 2% (5m)` / `Throttles > 0`
- `DynamoDB ThrottledRequests > 0`
- `DDB Streams IteratorAge > 60s`
- `DLQ OldestMessage > 60s`

---

**備考**: PITR（Point-In-Time Recovery）は要件に応じて有効化。保持期間/料金差異は最新仕様を確認。

