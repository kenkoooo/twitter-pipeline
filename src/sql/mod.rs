use crate::current_time_duration;
use anyhow::Result;
use async_trait::async_trait;
use egg_mode::user::TwitterUser;
use sqlx::postgres::PgRow;
use sqlx::types::Json;
use sqlx::{PgPool, Row};

const FRIENDS_IDS: &str = "friends_ids";
const FOLLOWERS_IDS: &str = "followers_ids";

#[async_trait]
pub trait PgPoolExt {
    async fn put_user_ids(&self, ids: &[u64], follower: bool) -> Result<()>;
    async fn get_user_ids(&self, follower: bool, confirmed_after: i64) -> Result<Vec<i64>>;

    async fn get_user_info(&self, id: i64) -> Result<Option<TwitterUser>>;
    async fn put_user_info(&self, user: &TwitterUser) -> Result<()>;

    async fn get_no_data_user_ids(&self, confirmed_after: i64, size: i64) -> Result<Vec<i64>>;
}

#[async_trait]
impl PgPoolExt for PgPool {
    async fn put_user_ids(&self, ids: &[u64], follower: bool) -> Result<()> {
        let table_name = if follower { FOLLOWERS_IDS } else { FRIENDS_IDS };
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
        let unixtime_second = current_time_duration().as_secs();

        const CHUNK_SIZE: usize = 1000;
        let ids = ids.iter().map(|&id| id as i64).collect::<Vec<_>>();
        for ids in ids.chunks(CHUNK_SIZE) {
            sqlx::query(&query)
                .bind(ids)
                .bind(unixtime_second as i64)
                .execute(self)
                .await?;
        }
        Ok(())
    }
    async fn get_user_ids(&self, follower: bool, confirmed_after: i64) -> Result<Vec<i64>> {
        let table_name = if follower { FOLLOWERS_IDS } else { FRIENDS_IDS };
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

    async fn get_user_info(&self, id: i64) -> Result<Option<TwitterUser>> {
        let result = sqlx::query(
            r"
        SELECT data FROM user_data WHERE id=$1
        ",
        )
        .bind(id)
        .try_map(|row: PgRow| row.try_get::<Json<TwitterUser>, _>("data"))
        .fetch_optional(self)
        .await;
        match result {
            Ok(result) => Ok(result.map(|x| x.0)),
            Err(e) => {
                log::error!("Failed to parse user_data of id={}: {:?}", id, e);
                Ok(None)
            }
        }
    }

    async fn put_user_info(&self, user: &TwitterUser) -> Result<()> {
        let id = user.id as i64;
        sqlx::query(
            r"
            INSERT INTO user_data (id, data) VALUES ($1, $2)
            ON CONFLICT (id)
            DO UPDATE SET data = EXCLUDED.data
        ",
        )
        .bind(id)
        .bind(Json(user))
        .execute(self)
        .await?;
        Ok(())
    }

    async fn get_no_data_user_ids(&self, confirmed_after: i64, size: i64) -> Result<Vec<i64>> {
        let mut no_data_friends_ids = sqlx::query(
            r"
            SELECT friends_ids.id FROM friends_ids
            LEFT JOIN user_data ON user_data.id = friends_ids.id
            WHERE user_data.data IS NULL AND confirmed_at > $1
            LIMIT $2
        ",
        )
        .bind(confirmed_after)
        .bind(size)
        .try_map(|row: PgRow| row.try_get::<i64, _>(0))
        .fetch_all(self)
        .await?;
        let no_data_followers_ids = sqlx::query(
            r"
            SELECT followers_ids.id FROM followers_ids
            LEFT JOIN user_data ON user_data.id = followers_ids.id
            WHERE user_data.data IS NULL AND confirmed_at > $1
            LIMIT $2        ",
        )
        .bind(confirmed_after)
        .bind(size)
        .try_map(|row: PgRow| row.try_get::<i64, _>(0))
        .fetch_all(self)
        .await?;
        no_data_friends_ids.extend(no_data_followers_ids);
        Ok(no_data_friends_ids)
    }
}
