use crate::sql::PgPoolExt;
use crate::twitter::TwitterClient;
use actix_web::rt::task::JoinHandle;
use anyhow::Result;
use sqlx::PgPool;
use std::time::Duration;

pub struct UserIdSynchronizer {
    pub pool: PgPool,
    pub client: TwitterClient,
    pub follower: bool,
}

impl UserIdSynchronizer {
    pub fn run(self) -> JoinHandle<()> {
        actix::spawn(async move {
            let mut cursor = -1;
            loop {
                log::info!("Fetching ids ...");
                match fetch_and_put(&self.client, &self.pool, self.follower, cursor).await {
                    Ok(next_cursor) => {
                        cursor = next_cursor;
                        if cursor == 0 {
                            cursor = -1;
                        }
                    }
                    Err(e) => {
                        log::error!("{:?}", e);
                    }
                }

                let duration = Duration::from_secs(10);
                log::info!("Sleeping {}s ...", duration.as_secs());
                actix::clock::sleep(duration).await;
            }
        })
    }
}

async fn fetch_and_put(
    client: &TwitterClient,
    pool: &PgPool,
    follower: bool,
    cursor: i64,
) -> Result<i64> {
    let screen_name = client.screen_name.clone();
    let (ids, next_cursor) = client.fetch_ids(screen_name, cursor, follower).await?;
    log::info!("cursor={} fetched={}", cursor, ids.len());
    pool.put_user_ids(&ids, follower).await?;
    Ok(next_cursor)
}
