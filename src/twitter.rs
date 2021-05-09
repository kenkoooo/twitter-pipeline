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
    pub async fn fetch_ids(&self, request: FetchIdRequest) -> Result<FetchIdResponse> {
        let c = match request {
            FetchIdRequest::Friends {
                screen_name,
                cursor,
            } => {
                let mut c = friends_ids(screen_name, &self.token).with_page_size(5000);
                c.next_cursor = cursor;
                c
            }
            FetchIdRequest::Followers {
                screen_name,
                cursor,
            } => {
                let mut c = followers_ids(screen_name, &self.token).with_page_size(5000);
                c.next_cursor = cursor;
                c
            }
        };
        match TwitterApiResponse::from(c.call().await) {
            TwitterApiResponse::Data(response) => {
                let ids = response.response.ids;
                let next_cursor = response.response.next_cursor;
                Ok(FetchIdResponse::Ids { ids, next_cursor })
            }
            TwitterApiResponse::RateLimitError(time) => {
                Ok(FetchIdResponse::RateLimitExceeded(time))
            }
            TwitterApiResponse::Error(e) => Err(e.into()),
        }
    }
    pub async fn get_relations(&self, user_ids: &[u64], wait: bool) -> Result<Vec<RelationLookup>> {
        wait_and_call(|| relation_lookup(user_ids.to_vec(), &self.token), wait).await
    }

    pub async fn get_user_data(&self, user_ids: &[u64], wait: bool) -> Result<Vec<TwitterUser>> {
        wait_and_call(|| lookup(user_ids.to_vec(), &self.token), wait).await
    }
}

async fn wait_and_call<F, T, Fut>(f: F, wait: bool) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: Future<Output = egg_mode::error::Result<egg_mode::Response<T>>>,
{
    loop {
        match TwitterApiResponse::from(f().await) {
            TwitterApiResponse::Data(response) => {
                return Ok(response.response);
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
                    return Err(anyhow::anyhow!("Rate Limit Exceeded: relation_lookup"));
                }
            }
            TwitterApiResponse::Error(e) => {
                return Err(e.into());
            }
        }
    }
}

#[derive(Debug)]
pub enum FetchIdRequest {
    Friends { screen_name: String, cursor: i64 },
    Followers { screen_name: String, cursor: i64 },
}

pub enum FetchIdResponse {
    RateLimitExceeded(SystemTime),
    Ids { ids: Vec<u64>, next_cursor: i64 },
}

pub enum TwitterApiResponse<T> {
    Data(T),
    RateLimitError(SystemTime),
    Error(egg_mode::error::Error),
}

impl<T> TwitterApiResponse<T> {
    pub fn map<U, F: Fn(T) -> U>(self, mapper: F) -> TwitterApiResponse<U> {
        match self {
            TwitterApiResponse::Data(t) => TwitterApiResponse::Data(mapper(t)),
            TwitterApiResponse::RateLimitError(time) => TwitterApiResponse::RateLimitError(time),
            TwitterApiResponse::Error(e) => TwitterApiResponse::Error(e),
        }
    }
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

pub trait RelationLookupExt {
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
