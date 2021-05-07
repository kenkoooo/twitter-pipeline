use anyhow::Result;
use egg_mode::error::Error::RateLimit;
use egg_mode::user::{followers_ids, friends_ids, relation_lookup, Connection, RelationLookup};
use egg_mode::Token;
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
        match GracefulResponse::from(c.call().await) {
            GracefulResponse::Response(response) => {
                let ids = response.response.ids;
                let next_cursor = response.response.next_cursor;
                Ok(FetchIdResponse::Ids { ids, next_cursor })
            }
            GracefulResponse::RateLimitError(time) => Ok(FetchIdResponse::RateLimitExceeded(time)),
            GracefulResponse::Error(e) => Err(e.into()),
        }
    }
    pub async fn get_relations(&self, user_ids: &[u64]) -> Result<Vec<RelationLookup>> {
        let response = relation_lookup(user_ids.to_vec(), &self.token)
            .await?
            .response;
        Ok(response)
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

enum GracefulResponse<T> {
    Response(T),
    RateLimitError(SystemTime),
    Error(egg_mode::error::Error),
}

impl<T> From<egg_mode::error::Result<T>> for GracefulResponse<T> {
    fn from(response: egg_mode::error::Result<T>) -> Self {
        match response {
            Ok(response) => GracefulResponse::Response(response),
            Err(RateLimit(reset)) => {
                let system_time = UNIX_EPOCH + Duration::from_secs(reset as u64);
                GracefulResponse::RateLimitError(system_time)
            }
            Err(e) => GracefulResponse::Error(e),
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
