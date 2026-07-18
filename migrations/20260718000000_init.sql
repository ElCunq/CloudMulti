CREATE TABLE IF NOT EXISTS cloudflare_accounts (
    id TEXT PRIMARY KEY,
    account_name TEXT NOT NULL,
    encrypted_token TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS zone_account_cache (
    zone_id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(account_id) REFERENCES cloudflare_accounts(id) ON DELETE CASCADE
);
