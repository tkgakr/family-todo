use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// トレーシングサブスクライバーを初期化
/// X-Ray統合は環境変数とLambdaランタイムで自動的に処理される
pub fn init_tracing() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 構造化ログ出力でCloudWatchに送信
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer().with_target(false).json(), // JSON形式でCloudWatchに出力
        )
        .with(EnvFilter::from_default_env())
        .try_init()?;

    Ok(())
}

/// Lambda 関数終了時のクリーンアップ（プレースホルダー）
pub fn shutdown_telemetry() {
    // 現在の実装では特別なクリーンアップは不要
    // X-Ray統合はLambdaランタイムが自動処理
}
