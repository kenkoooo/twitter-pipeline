use anyhow::Result;
use async_trait::async_trait;
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};

const FRIENDS_IDS: &str = "friends_ids";
const FOLLOWERS_IDS: &str = "followers_ids";

pub struct UserIdEntry {
    pub id: i64,
    pub confirmed_at: i64,
    pub created_at: i64,
}

#[async_trait]
pub trait UserIdClient {
    async fn get_all_user_id_entries(&self, follower: bool) -> Result<Vec<UserIdEntry>>;
}

#[async_trait]
impl UserIdClient for PgPool {
    async fn get_all_user_id_entries(&self, follower: bool) -> Result<Vec<UserIdEntry>> {
        let table_name = if follower { FOLLOWERS_IDS } else { FRIENDS_IDS };
        let query = format!(
            r"
            SELECT id, confirmed_at, created_at FROM {table_name}
        ",
            table_name = table_name
        );
        let ids = sqlx::query(&query)
            .try_map(|row: PgRow| {
                let id: i64 = row.try_get("id")?;
                let confirmed_at: i64 = row.try_get("confirmed_at")?;
                let created_at: i64 = row.try_get("created_at")?;
                Ok(UserIdEntry {
                    id,
                    confirmed_at,
                    created_at,
                })
            })
            .fetch_all(self)
            .await?;
        Ok(ids)
    }
}
