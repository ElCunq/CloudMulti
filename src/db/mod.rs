use std::path::Path;
use std::str::FromStr;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct AccountRecord {
    pub id: String,
    pub account_name: String,
    pub encrypted_token: String,
    pub created_at: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct AccountSummaryRecord {
    pub id: String,
    pub account_name: String,
    pub created_at: String,
}

#[derive(Clone)]
pub struct Db {
    pub pool: SqlitePool,
}

impl Db {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn insert_account(
        &self,
        id: &str,
        account_name: &str,
        encrypted_token: &str,
    ) -> Result<AccountRecord, sqlx::Error> {
        let record = sqlx::query_as::<_, AccountRecord>(
            r#"
            INSERT INTO cloudflare_accounts (id, account_name, encrypted_token)
            VALUES ($1, $2, $3)
            RETURNING id, account_name, encrypted_token, COALESCE(created_at, CURRENT_TIMESTAMP) as created_at
            "#
        )
        .bind(id)
        .bind(account_name)
        .bind(encrypted_token)
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    pub async fn list_accounts(&self) -> Result<Vec<AccountSummaryRecord>, sqlx::Error> {
        let records = sqlx::query_as::<_, AccountSummaryRecord>(
            r#"
            SELECT id, account_name, COALESCE(created_at, CURRENT_TIMESTAMP) as created_at
            FROM cloudflare_accounts
            ORDER BY created_at DESC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    pub async fn get_account(&self, id: &str) -> Result<Option<AccountRecord>, sqlx::Error> {
        let record = sqlx::query_as::<_, AccountRecord>(
            r#"
            SELECT id, account_name, encrypted_token, COALESCE(created_at, CURRENT_TIMESTAMP) as created_at
            FROM cloudflare_accounts
            WHERE id = $1
            "#
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    pub async fn delete_account(&self, id: &str) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            r#"
            DELETE FROM cloudflare_accounts
            WHERE id = $1
            "#
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    pub async fn list_all_accounts_with_tokens(&self) -> Result<Vec<AccountRecord>, sqlx::Error> {
        let records = sqlx::query_as::<_, AccountRecord>(
            r#"
            SELECT id, account_name, encrypted_token, COALESCE(created_at, CURRENT_TIMESTAMP) as created_at
            FROM cloudflare_accounts
            ORDER BY created_at DESC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    pub async fn upsert_zone_cache(&self, zone_id: &str, account_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO zone_account_cache (zone_id, account_id, updated_at)
            VALUES ($1, $2, CURRENT_TIMESTAMP)
            ON CONFLICT(zone_id) DO UPDATE SET
                account_id = excluded.account_id,
                updated_at = CURRENT_TIMESTAMP
            "#
        )
        .bind(zone_id)
        .bind(account_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_account_for_zone(&self, zone_id: &str) -> Result<Option<AccountRecord>, sqlx::Error> {
        let record = sqlx::query_as::<_, AccountRecord>(
            r#"
            SELECT c.id, c.account_name, c.encrypted_token, COALESCE(c.created_at, CURRENT_TIMESTAMP) as created_at
            FROM zone_account_cache z
            JOIN cloudflare_accounts c ON z.account_id = c.id
            WHERE z.zone_id = $1
            "#
        )
        .bind(zone_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }
}

pub async fn init_db(database_url: &str) -> Result<SqlitePool, sqlx::Error> {
    // If the database URL starts with sqlite:// and has a file path, ensure the parent folder exists
    if let Some(path_str) = database_url.strip_prefix("sqlite://") {
        let path = path_str.split('?').next().unwrap_or(path_str);
        if path != ":memory:" && !path.is_empty() {
            if let Some(parent) = Path::new(path).parent() {
                if !parent.as_os_str().is_empty() {
                    let _ = std::fs::create_dir_all(parent);
                }
            }
        }
    }

    let connect_options = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(10)
        .connect_with(connect_options)
        .await?;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await?;

    Ok(pool)
}
