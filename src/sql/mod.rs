use anyhow::Result;
use async_trait::async_trait;
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use std::time::{SystemTime, UNIX_EPOCH};

pub mod queue;

const FRIENDS_IDS: &str = "friends_ids";
const FOLLOWERS_IDS: &str = "followers_ids";

pub enum PutIdsRequest {
    Friends(Vec<u64>),
    Followers(Vec<u64>),
}

#[async_trait]
pub trait PgPoolExt {
    async fn put_user_ids(&self, request: &PutIdsRequest) -> Result<()>;
    async fn get_user_ids(&self, confirmed_after: i64, is_friends: bool) -> Result<Vec<i64>>;
    async fn put_white_list(&self, id: i64) -> Result<()>;
}

#[async_trait]
impl PgPoolExt for PgPool {
    async fn put_user_ids(&self, request: &PutIdsRequest) -> Result<()> {
        let table_name = match request {
            PutIdsRequest::Friends(_) => FRIENDS_IDS,
            PutIdsRequest::Followers(_) => FOLLOWERS_IDS,
        };
        let query = format!(
            r"
            INSERT INTO {table_name} (id, confirmed_at)
            VALUES (
                UNNEST($1::BIGINT[]),
                $2
            )
            ON CONFLICT (id)
            DO UPDATE SET confirmed_at = EXCLUDED.confirmed_at
        ",
            table_name = table_name
        );
        let unixtime_second = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

        const CHUNK_SIZE: usize = 1000;
        let ids = match request {
            PutIdsRequest::Friends(ids) => ids.iter().map(|&id| id as i64).collect::<Vec<_>>(),
            PutIdsRequest::Followers(ids) => ids.iter().map(|&id| id as i64).collect::<Vec<_>>(),
        };
        for ids in ids.chunks(CHUNK_SIZE) {
            sqlx::query(&query)
                .bind(ids)
                .bind(unixtime_second as i64)
                .execute(self)
                .await?;
        }
        Ok(())
    }
    async fn get_user_ids(&self, confirmed_after: i64, is_friends: bool) -> Result<Vec<i64>> {
        let table_name = if is_friends {
            FRIENDS_IDS
        } else {
            FOLLOWERS_IDS
        };
        let query = format!(
            r"
            SELECT id FROM {table_name}
            WHERE confirmed_at > $1
        ",
            table_name = table_name
        );
        let ids = sqlx::query(&query)
            .bind(confirmed_after)
            .try_map(|row: PgRow| row.try_get::<i64, _>("id"))
            .fetch_all(self)
            .await?;
        Ok(ids)
    }
    async fn put_white_list(&self, id: i64) -> Result<()> {
        sqlx::query(
            r"
            INSERT INTO whitelist (id) VALUES ($1) ON CONFLICT DO NOTHING",
        )
        .bind(id)
        .execute(self)
        .await?;
        Ok(())
    }
}
