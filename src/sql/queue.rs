use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgRow;
use sqlx::types::Json;
use sqlx::{PgPool, Row};

#[async_trait]
pub trait MessageQueue {
    async fn push_message(&self, msg: Message) -> Result<()>;
    async fn pop_message(&self) -> Result<Option<Message>>;
}

#[async_trait]
impl MessageQueue for PgPool {
    async fn push_message(&self, msg: Message) -> Result<()> {
        sqlx::query(
            r"
            INSERT INTO message_queue (data) VALUES ($1)
        ",
        )
        .bind(Json(msg))
        .execute(self)
        .await?;
        Ok(())
    }
    async fn pop_message(&self) -> Result<Option<Message>> {
        let response = sqlx::query(
            r"
            SELECT id, data FROM message_queue 
            WHERE pg_try_advisory_lock(tableoid::int, id)
            LIMIT 1
        ",
        )
        .try_map(|row: PgRow| {
            let id: i32 = row.try_get("id")?;
            let data: Json<Message> = row.try_get("data")?;
            Ok((id, data))
        })
        .fetch_optional(self)
        .await?;
        if let Some(response) = response {
            let (id, message) = response;
            sqlx::query(
                r"
            DELETE FROM message_queue WHERE id=$1 returning pg_advisory_unlock(tableoid::int, id)
        ",
            )
            .bind(id)
            .execute(self)
            .await?;
            Ok(Some(message.0))
        } else {
            Ok(None)
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct Message {
    pub user_id: u64,
    pub message_type: MessageType,
}

#[derive(Deserialize, Serialize)]
pub enum MessageType {
    Follow,
    Remove,
}

#[async_trait]
pub trait ConfirmationQueue {
    async fn push_remove_candidate(&self, user_id: u64) -> Result<()>;
    async fn pop_remove_candidate(&self) -> Result<Option<u64>>;
}

#[async_trait]
impl ConfirmationQueue for PgPool {
    async fn push_remove_candidate(&self, user_id: u64) -> Result<()> {
        sqlx::query(
            r"
            INSERT INTO confirmation_queue (user_id) VALUES ($1)
        ",
        )
        .bind(user_id as i64)
        .execute(self)
        .await?;
        Ok(())
    }

    async fn pop_remove_candidate(&self) -> Result<Option<u64>> {
        let response = sqlx::query(
            r"
            SELECT id, user_id FROM confirmation_queue 
            WHERE pg_try_advisory_lock(tableoid::int, id)
            LIMIT 1
        ",
        )
        .try_map(|row: PgRow| {
            let id: i32 = row.try_get("id")?;
            let user_id: i64 = row.try_get("user_id")?;
            Ok((id, user_id))
        })
        .fetch_optional(self)
        .await?;

        if let Some((id, user_id)) = response {
            sqlx::query(
                r"
        DELETE FROM confirmation_queue WHERE id=$1 returning pg_advisory_unlock(tableoid::int, id)
        ",
            )
            .bind(id)
            .execute(self)
            .await?;
            Ok(Some(user_id as u64))
        } else {
            Ok(None)
        }
    }
}
