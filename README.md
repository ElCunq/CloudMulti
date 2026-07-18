# Self-Hosted Multi-Account Cloudflare Dashboard Backend

A secure, lightweight (<30MB RAM footprint), high-performance self-hosted multi-account Cloudflare Dashboard backend built with **Rust (Axum + Tokio)**, **SQLx (SQLite)**, **Reqwest**, and **AES-256-GCM**. Designed specifically for single-container homelab/VPS deployments managed via **Coolify** or **Docker**.

---

## Security & Architecture

- **AES-256-GCM Token Encryption**: Cloudflare API tokens are encrypted with a 32-byte server-side master key before being persisted to SQLite (`cloudflare_accounts`). Tokens are never stored in plaintext and are never leaked via API endpoints.
- **Automatic Token Verification**: When saving an account, the backend verifies the token via Cloudflare's `GET /user/tokens/verify` endpoint before saving.
- **Concurrent Zone Aggregation**: Fetches all domains across all saved accounts concurrently using `futures::future::join_all`.
- **Smart Zone-to-Account Resolution**: Endpoints operating on a specific `zone_id` (`/dns`, `/purge-cache`, `/ssl`) automatically resolve which account owns that zone using a local SQLite cache (`zone_account_cache`) with fallback to concurrent probing.
- **Zero Placeholders & Typed Errors**: Uses a structured `AppError` enum implementing `axum::response::IntoResponse` for clean JSON error payloads.

---

## Environment Variables

| Variable | Required | Default | Description |
| :--- | :---: | :--- | :--- |
| `ENCRYPTION_KEY` | No (Auto) | `Auto-generated` | Master encryption key (exactly 32 bytes or 64 hex characters). If omitted on first boot, the backend automatically generates one (`data/encryption_key`) and logs a banner. |
| `DATABASE_URL` | No | `sqlite://data/cloudflare.db?mode=rwc` | SQLite connection URL. The parent directory and migrations will be created automatically. |
| `PORT` | No | `8080` | HTTP listening port. |
| `RUST_LOG` | No | `info,cloudflare_dashboard_backend=debug` | Tracing log level. |

---

## API Endpoints

### 1. Account Management
- `POST /api/accounts`
  - Request Body: `{ "account_name": "Personal", "api_token": "<cloudflare_token>" }`
  - Verifies token against Cloudflare (`status == "active"`), encrypts with AES-256-GCM, and inserts into DB.
- `GET /api/accounts`
  - Returns: `[{ "id": "uuid", "account_name": "Personal", "created_at": "..." }]` (Tokens never leaked).
- `DELETE /api/accounts/:id`
  - Deletes the account and its cached zone mappings from SQLite.

### 2. Unified Domain (Zone) Management
- `GET /api/zones`
  - Concurrently queries `GET /zones` across ALL stored accounts.
  - Returns unified JSON array: `[{ "account_name": "Personal", "account_id": "uuid", "name": "example.com", "status": "active", "id": "zone_id", "name_servers": [...] }]`
- `POST /api/zones`
  - Request Body: `{ "account_id": "uuid", "domain_name": "newdomain.com", "cf_account_id": "optional" }`
  - If `cf_account_id` is omitted, the backend automatically calls `GET /accounts` using the decrypted token to find the Cloudflare account ID.

### 3. DNS Record Management (Per Zone)
- `GET /api/zones/:zone_id/dns`
  - Returns all DNS records for `zone_id`.
- `POST /api/zones/:zone_id/dns`
  - Request Body: `{ "type": "A", "name": "sub.example.com", "content": "1.2.3.4", "ttl": 1, "proxied": true }`
- `DELETE /api/zones/:zone_id/dns/:record_id`
  - Deletes the specified DNS record.

### 4. Cache & Quick Operations
- `POST /api/zones/:zone_id/purge-cache`
  - Instantly clears edge cache (`{"purge_everything": true}`).
- `PATCH /api/zones/:zone_id/ssl`
  - Request Body: `{ "value": "full" }` (Valid values: `off`, `flexible`, `full`, `strict`).

---

## Coolify & Docker Deployment

The project includes a multi-stage `Dockerfile` utilizing `cargo-chef` for ultra-fast dependency caching and low memory footprint (<30MB RAM):

### Deploy via Coolify
1. Create a new service in Coolify from your Git repository (or Dockerfile).
2. Add Persistent Storage (Recommended):
   - Mount `/app/data` to persist your `cloudflare.db` and master encryption key across redeployments.
3. Expose port `8080`.
4. **First Boot & Environment Variables:**
   - If `ENCRYPTION_KEY` is not set on first deploy, the backend auto-generates a secure 256-bit key, saves it to `/app/data/encryption_key`, and outputs a prominent banner in your **Coolify Deployment Logs**.
   - *Tip:* Copy the generated `ENCRYPTION_KEY=...` from the logs and paste it into Coolify's **Environment Variables** panel to easily view and manage it.

### Deploy via Docker CLI / Docker Compose
```bash
# Build the multi-stage image
docker build -t cloudflare-dashboard-backend .

# Run container with persistent data folder
docker run -d \
  -p 8080:8080 \
  -e ENCRYPTION_KEY=$(openssl rand -hex 32) \
  -v $(pwd)/data:/app/data \
  --name cloudflare-dashboard \
  cloudflare-dashboard-backend
```
