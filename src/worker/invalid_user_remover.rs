use crate::current_time_duration;
use crate::sql::PgPoolExt;
use crate::twitter::{RelationLookupExt, TwitterClient};
use actix::clock::sleep;
use actix_web::rt::task::JoinHandle;
use anyhow::Result;
use egg_mode::user::{unfollow, TwitterUser};
use std::collections::BTreeSet;
use std::iter::FromIterator;
use std::time::Duration;

pub struct InvalidUserRemover<P> {
    pub pool: P,
    pub client: TwitterClient,
}

impl<P: PgPoolExt + 'static> InvalidUserRemover<P> {
    pub fn start(self) -> JoinHandle<()> {
        actix::spawn(async move {
            loop {
                if let Err(e) = extract_and_unfollow(&self.pool, &self.client).await {
                    log::error!("{:?}", e);
                }
                let duration = Duration::from_secs(300);
                log::info!("Sleeping {} seconds", duration.as_secs());
                sleep(duration).await;
            }
        })
    }
}

async fn extract_and_unfollow<P: PgPoolExt>(pool: &P, client: &TwitterClient) -> Result<()> {
    let one_hour_ago = current_time_duration().as_secs() - 3600;
    let followers = BTreeSet::from_iter(
        pool.get_user_ids(one_hour_ago as i64, false)
            .await?
            .into_iter(),
    );
    let friends = pool.get_user_ids(one_hour_ago as i64, true).await?;

    let non_followers = friends
        .into_iter()
        .filter(|user_id| !followers.contains(user_id))
        .collect::<Vec<_>>();
    let mut no_data_user_ids = vec![];
    let mut non_followers_data = vec![];
    for user_id in non_followers {
        if let Some(user_data) = pool.get_user_info(user_id).await? {
            non_followers_data.push(user_data);
        } else {
            no_data_user_ids.push(user_id as u64);
            if no_data_user_ids.len() == 100 {
                break;
            }
        }
    }

    if !no_data_user_ids.is_empty() {
        let data = client.get_user_data(&no_data_user_ids).await?;
        for data in data.iter() {
            pool.put_user_info(data).await?;
        }
        non_followers_data.extend(data);
    }

    let invalid_user_ids = non_followers_data
        .into_iter()
        .filter(|user| is_invalid_user(user))
        .take(100)
        .map(|user| user.id)
        .collect::<Vec<_>>();
    let relations = client.get_relations(&invalid_user_ids).await?;
    let user_ids = relations
        .into_iter()
        .filter(|relation| relation.is_friend() && !relation.is_follower())
        .map(|relation| relation.id)
        .collect::<Vec<_>>();

    for user_id in user_ids {
        log::info!("Unfollowing {}", user_id);
        let response = unfollow(user_id, &client.token).await?;
        log::info!("Unfollowed @{}", response.response.screen_name);

        let sleep_duration = Duration::from_secs(300);
        log::info!("Sleeping {} seconds", sleep_duration.as_secs());
        sleep(sleep_duration).await;
    }
    Ok(())
}

fn is_invalid_user(user: &TwitterUser) -> bool {
    if user.friends_count > 0 {
        return false;
    }
    user.status
        .as_ref()
        .map(|status| {
            let now = current_time_duration().as_secs() as i64;
            let timestamp = status.created_at.timestamp();
            (now - timestamp) > 3600 * 24 * 365
        })
        .unwrap_or(false)
}
