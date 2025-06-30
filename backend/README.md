# Family Todo バックエンド

このプロジェクトは、Rustで実装されたAWS Lambda関数を使用したFamily Todoアプリケーションのバックエンドです。

## プロジェクト構成

- **単一環境**: dev/prodの区別なく、単一環境で運用します
- **主要リソース**: 
  - Lambda（Rust）
  - API Gateway
  - DynamoDB（今後実装予定）
- **リソース名**:
  - Lambda関数: todo-handler
  - DynamoDBテーブル: todo-table
  - API Gatewayステージ: API

## 前提条件

- [Rust](https://www.rust-lang.org/tools/install)
- [Cargo Lambda](https://www.cargo-lambda.info/guide/installation.html)

## ビルド方法

本番用にプロジェクトをビルドするには、以下のコマンドを実行します：

```bash
cargo lambda build --release
```

開発用にビルドする場合は、`--release`フラグを省略してください。

Lambda関数のビルドについて詳しくは、[Cargo Lambdaのドキュメント](https://www.cargo-lambda.info/commands/build.html)を参照してください。

## テスト方法

### Rustユニットテスト

通常のRustユニットテストは次のコマンドで実行できます：

```bash
cargo test
```

### Cargo Lambdaを使用したローカルテスト

ローカルで統合テストを実行したい場合は、`cargo lambda watch`と`cargo lambda invoke`コマンドを使用できます。

1. まず、`cargo lambda watch`を実行してローカルサーバーを起動します。コードに変更を加えると、サーバーは自動的に再起動します。

2. 次に、Lambda関数にイベントデータを渡す方法が必要です。

API Gatewayリクエストなど、サポートされているイベントタイプを使用している場合、Rust Runtimeリポジトリにある[イベントペイロード](https://github.com/awslabs/aws-lambda-rust-runtime/tree/main/lambda-events/src/fixtures)を利用できます。

これらのサンプルは`--data-example`フラグを使って直接利用できます。値は[lambda-events](https://github.com/awslabs/aws-lambda-rust-runtime/tree/main/lambda-events/src/fixtures)リポジトリ内のファイル名から`example_`プレフィックスと`.json`拡張子を除いたものです。

```bash
cargo lambda invoke --data-example apigw-request
```

独自のイベントデータ構造を定義する場合は、テストしたいデータを含むJSONファイルを作成できます。例：

```json
{
    "command": "test"
}
```

その後、`cargo lambda invoke --data-file ./data.json`を実行して、`data.json`内のデータを使用して関数を呼び出します。

HTTPイベントの場合、cURLや他のHTTPクライアントを使用して関数を直接呼び出すこともできます。例：

```bash
curl http://localhost:9000
```

ローカルサーバーの実行について詳しくは、[`watch`コマンドのCargo Lambdaドキュメント](https://www.cargo-lambda.info/commands/watch.html)を参照してください。
関数の呼び出しについて詳しくは、[`invoke`コマンドのCargo Lambdaドキュメント](https://www.cargo-lambda.info/commands/invoke.html)を参照してください。

### AWS SAMを使用したビルドとテスト

SAMテンプレートを使用して、ローカル環境でLambda関数とAPI Gatewayをエミュレートしてテストすることもできます。

#### 前提条件

- [AWS SAM CLI](https://docs.aws.amazon.com/serverless-application-model/latest/developerguide/serverless-sam-cli-install.html)のインストール
- SAMテンプレートファイル（template.yaml）の設定

#### SAMを使用したビルド

```bash
cd ../infra
sam build --beta-features
```

このコマンドは、SAMテンプレートに定義されている全てのリソースをビルドします。Rustプロジェクトの場合、ビルド処理は、SAMテンプレートで指定されたコンテナシステムで実行されます。Rustのビルドコマンドは自動的に実行されます。

特定の関数のみをビルドする場合は、次のように指定します：

```bash
sam build todoHandler --beta-features
```

#### ローカルでLambda関数を実行

ビルド後、ローカルでLambda関数を実行できます：

```bash
sam local invoke todoHandler --event events/event.json
```

#### ローカルでAPI Gatewayをエミュレート

```bash
sam local start-api
```

これにより、`http://127.0.0.1:3000/`でAPIエンドポイントにアクセスできます。

> ⚠️ ポート3000が既に使用中の場合は、他のプロセスを停止するか、`--port`オプションで別のポートを指定してください。

#### テストイベントの作成

`events`ディレクトリに、テスト用のイベントJSONファイルを作成します：

```json
{
  "httpMethod": "GET",
  "path": "/todos",
  "queryStringParameters": {
    "name": "test"
  }
}
```

> ⚠️ RustランタイムのLambda関数でAPI Gatewayイベントを正しく取り扱うためには、[lambda-eventsリポジトリ](https://github.com/awslabs/aws-lambda-rust-runtime/tree/main/lambda-events/src/fixtures)の`apigw-request.json`など、公式サンプルを参考にしてください。
> イベント形式が異なる場合、デシリアライズエラーが発生します。

#### APIのエンドツーエンドテスト

SAM localでAPIを起動したら、cURLでテストできます：

```bash
curl http://127.0.0.1:3000/todos?name=test
```

これにより、Lambda関数からのレスポンスが返ってきます。

SAMを使ったテストの詳細については、[AWS SAM CLIのドキュメント](https://docs.aws.amazon.com/serverless-application-model/latest/developerguide/serverless-sam-cli-using-debugging.html)を参照してください。

## デプロイ方法

SAMテンプレートを使用して、AWS環境にデプロイします。テンプレートは固定名を使用しており、環境変数による区別はありません。

基本的なデプロイ手順については、今後更新予定です。

## 現在の実装状況

- Lambda HTTP関数の基本的な実装が完了
- HTTP GETリクエストに対する基本的なレスポンス処理
- テストケースの実装
- DynamoDBとの連携は今後実装予定
