//! todo-api バイナリのエントリポイント
//! ローカル開発用に HTTP サーバを起動します。

use todo_api::app;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    // 簡易なロガー設定（RUST_LOG 環境変数で制御可能）
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .init();

    // ポートは環境変数 PORT（なければ 3000）
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3000);
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind address");
    tracing::info!(%addr, "server starting");

    // ルータを構築して提供
    let router = app();
    axum::serve(listener, router)
        .await
        .expect("server error");
}
