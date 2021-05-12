use actix::clock::sleep;
use anyhow::Result;
use egg_mode::error::Error::RateLimit;
use egg_mode::user::{
    followers_ids, friends_ids, lookup, relation_lookup, Connection, RelationLookup, TwitterUser,
};
use egg_mode::Token;
use std::future::Future;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Clone)]
pub struct TwitterClient {
    pub token: Token,
    pub screen_name: String,
}

impl TwitterClient {
    pub(crate) async fn fetch_ids(
        &self,
        screen_name: String,
        cursor: i64,
        follower: bool,
    ) -> Result<(Vec<u64>, i64)> {
        let c = if follower {
            followers_ids(screen_name, &self.token)
        } else {
            friends_ids(screen_name, &self.token)
        };
        let mut c = c.with_page_size(5000);
        c.next_cursor = cursor;
        let response = wait_and_call(|| c.call(), true, "fetch_ids")
            .await?
            .response;
        Ok((response.ids, response.next_cursor))
    }
    pub(crate) async fn get_relations(
        &self,
        user_ids: &[u64],
        wait: bool,
    ) -> Result<Vec<RelationLookup>> {
        wait_and_call(
            || relation_lookup(user_ids.to_vec(), &self.token),
            wait,
            "relation_lookup",
        )
        .await
        .map(|response| response.response)
    }

    pub(crate) async fn get_user_data(
        &self,
        user_ids: &[u64],
        wait: bool,
    ) -> Result<Vec<TwitterUser>> {
        log::info!("Fetching data of {} users", user_ids.len());
        wait_and_call(|| lookup(user_ids.to_vec(), &self.token), wait, "lookup")
            .await
            .map(|response| response.response)
    }
}

async fn wait_and_call<F, T, Fut>(f: F, wait: bool, api_name: &str) -> Result<egg_mode::Response<T>>
where
    F: Fn() -> Fut,
    Fut: Future<Output = egg_mode::error::Result<egg_mode::Response<T>>>,
{
    loop {
        match TwitterApiResponse::from(f().await) {
            TwitterApiResponse::Data(response) => {
                return Ok(response);
            }
            TwitterApiResponse::RateLimitError(time) => {
                if wait {
                    let now = SystemTime::now();
                    if let Ok(duration) = time.duration_since(now) {
                        log::info!(
                            "Rate Limit Exceeded: Sleeping {} seconds",
                            duration.as_secs()
                        );
                        sleep(duration).await;
                    }
                } else {
                    log::error!("Rate Limit Exceeded");
                    return Err(anyhow::anyhow!("Rate Limit Exceeded: {}", api_name));
                }
            }
            TwitterApiResponse::Error(e) => {
                return Err(e.into());
            }
        }
    }
}

enum TwitterApiResponse<T> {
    Data(T),
    RateLimitError(SystemTime),
    Error(egg_mode::error::Error),
}

impl<T> From<egg_mode::error::Result<T>> for TwitterApiResponse<T> {
    fn from(response: egg_mode::error::Result<T>) -> Self {
        match response {
            Ok(response) => TwitterApiResponse::Data(response),
            Err(RateLimit(reset)) => {
                let system_time = UNIX_EPOCH + Duration::from_secs(reset as u64);
                TwitterApiResponse::RateLimitError(system_time)
            }
            Err(e) => TwitterApiResponse::Error(e),
        }
    }
}

pub(crate) trait RelationLookupExt {
    fn is_friend(&self) -> bool;
    fn is_follower(&self) -> bool;
    fn is_pending(&self) -> bool;
}

impl RelationLookupExt for RelationLookup {
    fn is_friend(&self) -> bool {
        for connection in self.connections.iter() {
            if let Connection::Following = connection {
                return true;
            }
        }
        false
    }

    fn is_follower(&self) -> bool {
        for connection in self.connections.iter() {
            if let Connection::FollowedBy = connection {
                return true;
            }
        }
        false
    }
    fn is_pending(&self) -> bool {
        for connection in self.connections.iter() {
            if let Connection::FollowingRequested = connection {
                return true;
            }
        }
        false
    }
}
