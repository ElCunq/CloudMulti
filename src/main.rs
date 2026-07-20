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
        .unwrap_or_else(|_| "sqlite:///app/data/cloudflare.db?mode=rwc".to_string());
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

    let proxy_notice = format!(
        "\n================================================================================\n\
         [COOLIFY / REVERSE PROXY DOMAIN ROUTING CHECK]\n\
         Server is bound to 0.0.0.0:{port}\n\
         If you attached a custom domain in Coolify and the page does not open (502 / timeout / blank):\n\
         1. Coolify's Traefik proxy routes to container port 80 by default unless told otherwise.\n\
         2. In Coolify -> Service Settings -> set 'Ports Exposes' to exactly: {port}\n\
            OR in Coolify -> FQDN / Domains field -> append :{port} (e.g. https://domain.com:{port})\n\
            OR add environment variable PORT=80 in Coolify so this backend listens on port 80.\n\
         ================================================================================"
    );
    tracing::info!("{}", proxy_notice);
    println!("{}", proxy_notice);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind to port");
    axum::serve(listener, app).await.expect("Server error");
}
