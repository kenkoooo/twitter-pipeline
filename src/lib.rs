use crate::sql::PgPoolExt;
use std::collections::BTreeSet;
use std::iter::FromIterator;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub mod server;
pub mod sql;
pub mod twitter;
pub mod worker;

pub fn current_time_duration() -> Duration {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Failed to get current UNIX time.")
}

pub(crate) async fn get_difference<P: PgPoolExt>(
    pool: &P,
    confirmed_after: i64,
    get_unfollowed_users: bool,
) -> anyhow::Result<Vec<i64>> {
    let followers = pool.get_user_ids(true, confirmed_after).await?;
    let friends = pool.get_user_ids(false, confirmed_after).await?;

    let followers = BTreeSet::from_iter(followers.into_iter());
    let friends = BTreeSet::from_iter(friends.into_iter());

    let result = if get_unfollowed_users {
        followers
            .into_iter()
            .filter(|follower| !friends.contains(follower))
            .collect()
    } else {
        friends
            .into_iter()
            .filter(|friend| !followers.contains(friend))
            .collect()
    };
    Ok(result)
}
