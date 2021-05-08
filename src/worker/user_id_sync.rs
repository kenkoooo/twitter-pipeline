use crate::sql::{PgPoolExt, PutIdsRequest};
use crate::twitter::{FetchIdRequest, FetchIdResponse, TwitterClient};
use actix_web::rt::task::JoinHandle;
use anyhow::Result;
use sqlx::PgPool;
use std::time::{Duration, SystemTime};

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
    let request = if follower {
        FetchIdRequest::Followers {
            screen_name,
            cursor,
        }
    } else {
        FetchIdRequest::Friends {
            screen_name,
            cursor,
        }
    };
    let result = client.fetch_ids(request).await?;
    match result {
        FetchIdResponse::Ids { ids, next_cursor } => {
            log::info!("cursor={} fetched={}", cursor, ids.len());
            let request = if follower {
                PutIdsRequest::Followers(ids)
            } else {
                PutIdsRequest::Friends(ids)
            };
            pool.put_user_ids(&request).await?;
            Ok(next_cursor)
        }
        FetchIdResponse::RateLimitExceeded(until) => {
            let now = SystemTime::now();
            if let Ok(duration) = until.duration_since(now) {
                log::info!(
                    "RateLimitExceeded: sleep for {} seconds",
                    duration.as_secs()
                );
                actix::clock::sleep(duration).await;
            }
            Ok(cursor)
        }
    }
}
