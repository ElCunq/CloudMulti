use axum::{
    routing::{delete, get, patch, post},
    Router,
};
use tower_http::{
    cors::{Any, CorsLayer},
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use crate::handlers::{accounts, dns, quick_ops, zones, tunnels, settings, analytics};
use crate::state::AppState;

pub fn build_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let api_routes = Router::new()
        // Health check
        .route("/health", get(|| async { axum::Json(serde_json::json!({ "status": "ok" })) }))
        // Accounts
        .route("/accounts", post(accounts::create_account).get(accounts::list_accounts))
        .route("/accounts/:id", delete(accounts::delete_account))
        // Zones
        .route("/zones", get(zones::list_zones).post(zones::create_zone))
        // DNS Records per Zone
        .route("/zones/:zone_id/dns", get(dns::list_dns_records).post(dns::create_dns_record))
        .route(
            "/zones/:zone_id/dns/:record_id",
            delete(dns::delete_dns_record)
                .put(dns::update_dns_record)
                .patch(dns::update_dns_record),
        )
        // Zone settings
        .route(
            "/zones/:zone_id/settings",
            get(settings::get_zone_settings).patch(settings::update_zone_settings),
        )
        // Cloudflare Tunnels (Zero Trust)
        .route("/tunnels", get(tunnels::list_all_tunnels))
        // GraphQL Analytics
        .route("/analytics", get(analytics::get_unified_analytics))
        // Quick Operations
        .route("/zones/:zone_id/purge-cache", post(quick_ops::purge_cache))
        .route("/zones/:zone_id/ssl", patch(quick_ops::update_ssl));

    let spa_service = ServeDir::new("public").fallback(ServeFile::new("public/index.html"));

    Router::new()
        .nest("/api", api_routes)
        .fallback_service(spa_service)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}
