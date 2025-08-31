# Family Todo App API ドキュメント

## 概要

Family Todo App は RESTful API を提供し、家族間での Todo 管理機能を実現しています。
イベントソーシングアーキテクチャに基づき、すべての操作がイベントとして記録され、完全な履歴追跡が可能です。

## 認証

現在は開発用にヘッダーベース認証を使用しています。

### 必須ヘッダー

```
X-Family-Id: 家族ID（ULID形式）
X-User-Id: ユーザーID（ULID形式）
```

本番環境では Amazon Cognito による JWT 認証に移行予定です。

## エンドポイント

### Base URL

開発環境: `http://localhost:3001/`
本番環境: `https://{api-gateway-domain}/{stage}/`

### Todo 管理

#### 1. Todo 一覧取得

**GET** `/todos`

家族のアクティブな Todo 一覧を取得します。

**Parameters:**
- `status` (query, optional): `active` (デフォルト) | `completed` | `all`
- `limit` (query, optional): 取得件数制限 (デフォルト: 50, 最大: 100)
- `cursor` (query, optional): ページネーション用カーソル

**Response: 200 OK**
```json
{
  "todos": [
    {
      "id": "01ARZ3NDEKTSV4RRFFQ69G5FAV",
      "title": "買い物リスト",
      "description": "週末の買い出し",
      "tags": ["家事", "急ぎ"],
      "completed": false,
      "created_by": "01ARZ3NDEKTSV4RRFFQ69G5FAV",
      "created_at": "2024-01-15T10:30:00Z",
      "updated_at": "2024-01-15T10:30:00Z",
      "version": 1
    }
  ],
  "next_cursor": "01ARZ3NDEKTSV4RRFFQ69G5FAW",
  "has_more": true
}
```

#### 2. Todo 詳細取得

**GET** `/todos/{id}`

指定された Todo の詳細情報を取得します。

**Parameters:**
- `id` (path, required): Todo ID (ULID形式)

**Response: 200 OK**
```json
{
  "id": "01ARZ3NDEKTSV4RRFFQ69G5FAV",
  "title": "買い物リスト",
  "description": "週末の買い出し",
  "tags": ["家事", "急ぎ"],
  "completed": false,
  "created_by": "01ARZ3NDEKTSV4RRFFQ69G5FAV",
  "created_at": "2024-01-15T10:30:00Z",
  "updated_at": "2024-01-15T10:30:00Z",
  "completed_at": null,
  "completed_by": null,
  "version": 1
}
```

**Response: 404 Not Found**
```json
{
  "error": "Todo not found",
  "message": "Todo with ID '01ARZ3NDEKTSV4RRFFQ69G5FAV' does not exist"
}
```

#### 3. Todo 作成

**POST** `/todos`

新しい Todo を作成します。

**Request Body:**
```json
{
  "title": "買い物リスト",
  "description": "週末の買い出し",
  "tags": ["家事", "急ぎ"]
}
```

**Response: 201 Created**
```json
{
  "id": "01ARZ3NDEKTSV4RRFFQ69G5FAV",
  "title": "買い物リスト",
  "description": "週末の買い出し",
  "tags": ["家事", "急ぎ"],
  "completed": false,
  "created_by": "01ARZ3NDEKTSV4RRFFQ69G5FAV",
  "created_at": "2024-01-15T10:30:00Z",
  "updated_at": "2024-01-15T10:30:00Z",
  "version": 1
}
```

**Validation Rules:**
- `title`: 必須、1-200文字
- `description`: オプション、最大1000文字
- `tags`: オプション、配列、最大10個、各タグ最大50文字

#### 4. Todo 更新

**PUT** `/todos/{id}`

既存の Todo を更新します。

**Parameters:**
- `id` (path, required): Todo ID (ULID形式)

**Request Body:**
```json
{
  "title": "買い物リスト（更新）",
  "description": "週末の買い出し（野菜多め）",
  "tags": ["家事", "急ぎ", "健康"]
}
```

**Response: 200 OK**
```json
{
  "id": "01ARZ3NDEKTSV4RRFFQ69G5FAV",
  "title": "買い物リスト（更新）",
  "description": "週末の買い出し（野菜多め）",
  "tags": ["家事", "急ぎ", "健康"],
  "completed": false,
  "created_by": "01ARZ3NDEKTSV4RRFFQ69G5FAV",
  "updated_by": "01ARZ3NDEKTSV4RRFFQ69G5FAV",
  "created_at": "2024-01-15T10:30:00Z",
  "updated_at": "2024-01-15T11:45:00Z",
  "version": 2
}
```

#### 5. Todo 完了

**POST** `/todos/{id}/complete`

Todo を完了状態にします。

**Parameters:**
- `id` (path, required): Todo ID (ULID形式)

**Response: 200 OK**
```json
{
  "id": "01ARZ3NDEKTSV4RRFFQ69G5FAV",
  "title": "買い物リスト",
  "description": "週末の買い出し",
  "tags": ["家事", "急ぎ"],
  "completed": true,
  "created_by": "01ARZ3NDEKTSV4RRFFQ69G5FAV",
  "completed_by": "01ARZ3NDEKTSV4RRFFQ69G5FAV",
  "created_at": "2024-01-15T10:30:00Z",
  "updated_at": "2024-01-15T12:00:00Z",
  "completed_at": "2024-01-15T12:00:00Z",
  "version": 3
}
```

#### 6. Todo 削除

**DELETE** `/todos/{id}`

Todo を削除します（論理削除）。

**Parameters:**
- `id` (path, required): Todo ID (ULID形式)

**Request Body (optional):**
```json
{
  "reason": "重複していたため"
}
```

**Response: 204 No Content**

#### 7. Todo 履歴取得

**GET** `/todos/{id}/history`

指定された Todo の変更履歴を取得します。

**Parameters:**
- `id` (path, required): Todo ID (ULID形式)
- `limit` (query, optional): 取得件数制限 (デフォルト: 50, 最大: 100)

**Response: 200 OK**
```json
{
  "todo_id": "01ARZ3NDEKTSV4RRFFQ69G5FAV",
  "events": [
    {
      "event_type": "todo_created_v2",
      "event_id": "01ARZ3NDEKTSV4RRFFQ69G5FAV",
      "todo_id": "01ARZ3NDEKTSV4RRFFQ69G5FAV",
      "title": "買い物リスト",
      "description": "週末の買い出し",
      "tags": ["家事", "急ぎ"],
      "created_by": "01ARZ3NDEKTSV4RRFFQ69G5FAV",
      "timestamp": "2024-01-15T10:30:00Z"
    },
    {
      "event_type": "todo_updated_v1",
      "event_id": "01ARZ3NDEKTSV4RRFFQ69G5FAW",
      "todo_id": "01ARZ3NDEKTSV4RRFFQ69G5FAV",
      "title": "買い物リスト（更新）",
      "description": "週末の買い出し（野菜多め）",
      "updated_by": "01ARZ3NDEKTSV4RRFFQ69G5FAV",
      "timestamp": "2024-01-15T11:45:00Z"
    }
  ]
}
```

### 家族管理

#### 8. 家族メンバー一覧取得

**GET** `/family/members`

家族メンバー一覧を取得します。

**Response: 200 OK**
```json
{
  "family_id": "01ARZ3NDEKTSV4RRFFQ69G5FAV",
  "members": [
    {
      "user_id": "01ARZ3NDEKTSV4RRFFQ69G5FAV",
      "name": "田中太郎",
      "email": "taro@example.com",
      "role": "admin",
      "joined_at": "2024-01-15T10:00:00Z"
    },
    {
      "user_id": "01ARZ3NDEKTSV4RRFFQ69G5FAW",
      "name": "田中花子",
      "email": "hanako@example.com",
      "role": "member",
      "joined_at": "2024-01-15T10:30:00Z"
    }
  ]
}
```

## エラーハンドリング

### HTTP ステータスコード

- `200`: 成功
- `201`: 作成成功
- `204`: 削除成功
- `400`: バリデーションエラー
- `401`: 認証エラー
- `403`: 認可エラー
- `404`: リソースが見つからない
- `409`: 競合状態（楽観的ロック失敗）
- `429`: レート制限超過
- `500`: サーバー内部エラー

### エラーレスポンス形式

```json
{
  "error": "validation_error",
  "message": "Title is required",
  "details": {
    "field": "title",
    "code": "required"
  },
  "request_id": "01ARZ3NDEKTSV4RRFFQ69G5FAV"
}
```

## レート制限

- **Command操作** (POST/PUT/DELETE): 10リクエスト/分/IP
- **Query操作** (GET): 60リクエスト/分/IP

制限に達すると `429 Too Many Requests` が返されます。

## CORS設定

以下のオリジンからのリクエストを許可：
- 開発環境: `http://localhost:3000`
- 本番環境: `https://family-todo-app.example.com`

## データ形式

### 日時形式

すべての日時は ISO 8601 形式 (RFC 3339) で表現されます。
例: `2024-01-15T10:30:00Z`

### ID形式

すべてのIDは26文字のULID形式で表現されます。
例: `01ARZ3NDEKTSV4RRFFQ69G5FAV`

ULIDは時系列順でソート可能で、作成時刻の情報を含みます。

## OpenAPI仕様

完全なAPI仕様は [openapi.yaml](./openapi.yaml) を参照してください。

## 変更履歴

### Version 2.0.0 (2024-01-15)
- イベントソーシング対応
- ULID採用
- 履歴取得API追加

### Version 1.0.0 (2024-01-01)
- 初期リリース
- 基本CRUD操作