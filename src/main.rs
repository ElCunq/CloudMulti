use cloudflare_dashboard_backend::{
    cloudflare::CloudflareClient,
    crypto::Crypto,
    db::{init_db, Db},
    routes::build_router,
    state::AppState,
};
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,cloudflare_dashboard_backend=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Cloudflare Dashboard Backend...");

    let crypto = match Crypto::from_env_or_auto() {
        Ok(c) => Arc::new(c),
        Err(e) => {
            tracing::error!("FATAL: Failed to initialize master encryption key: {}", e);
            std::process::exit(1);
        }
    };

    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite://data/cloudflare.db?mode=rwc".to_string());
    tracing::info!("Connecting to SQLite database at {}...", database_url);

    let pool = match init_db(&database_url).await {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("FATAL: Failed to initialize SQLite database: {}", e);
            std::process::exit(1);
        }
    };

    let db = Db::new(pool);
    let cf = CloudflareClient::new();

    let state = AppState { db, crypto, cf };
    let app = build_router(state);

    let port: u16 = env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind to port");
    axum::serve(listener, app).await.expect("Server error");
}
