use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use cloudflare_dashboard_backend::{
    cloudflare::CloudflareClient,
    crypto::Crypto,
    db::{init_db, Db},
    routes::build_router,
    state::AppState,
};
use std::sync::Arc;
use tower::ServiceExt; // for oneshot

#[tokio::test]
async fn test_health_check() {
    let pool = init_db("sqlite::memory:").await.expect("Failed to init in-memory DB");
    let db = Db::new(pool);
    let crypto = Arc::new(Crypto::from_key_str("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef").unwrap());
    let cf = CloudflareClient::new();
    let state = AppState { db, crypto, cf };
    let app = build_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_account_crud_and_endpoints() {
    let pool = init_db("sqlite::memory:").await.expect("Failed to init in-memory DB");
    let db = Db::new(pool);
    let crypto = Arc::new(Crypto::from_key_str("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef").unwrap());
    let cf = CloudflareClient::new();
    let state = AppState { db: db.clone(), crypto: crypto.clone(), cf };
    let app = build_router(state.clone());

    // 1. Check initially empty list via GET /api/accounts
    let response = app.clone()
        .oneshot(
            Request::builder()
                .uri("/api/accounts")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::OK);

    // 2. Insert account directly via DB layer (simulating verified token save)
    let enc_token = crypto.encrypt("my_fake_cf_token").unwrap();
    let record = db
        .insert_account("test-id-1", "My Personal Account", &enc_token)
        .await
        .expect("Failed to insert account");

    assert_eq!(record.id, "test-id-1");
    assert_eq!(record.account_name, "My Personal Account");

    // 3. GET /api/accounts should now return 1 account without leaking the token
    let response = app.clone()
        .oneshot(
            Request::builder()
                .uri("/api/accounts")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
    assert!(body_str.contains("My Personal Account"));
    assert!(!body_str.contains("my_fake_cf_token")); // Ensure token is not leaked!
    assert!(!body_str.contains(&enc_token)); // Ensure encrypted token is not leaked either!

    // 4. DELETE /api/accounts/test-id-1
    let response = app.clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/accounts/test-id-1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // 5. Verify account deleted
    let response = app.clone()
        .oneshot(
            Request::builder()
                .uri("/api/accounts")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("Failed to execute request");

    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
    assert_eq!(body_str, "[]");
}

#[tokio::test]
async fn test_invalid_token_verification() {
    let pool = init_db("sqlite::memory:").await.expect("Failed to init in-memory DB");
    let db = Db::new(pool);
    let crypto = Arc::new(Crypto::from_key_str("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef").unwrap());
    let cf = CloudflareClient::new();
    let state = AppState { db, crypto, cf };
    let app = build_router(state);

    // POST /api/accounts with invalid token should fail verification (HTTP 400 Bad Request)
    let payload = serde_json::json!({
        "account_name": "Test Account",
        "api_token": "invalid_token_xyz"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/accounts")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&payload).unwrap()))
                .unwrap(),
        )
        .await
        .expect("Failed to execute request");

    // Because 'invalid_token_xyz' is not valid on Cloudflare, we expect BAD_REQUEST (InvalidToken error)
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
