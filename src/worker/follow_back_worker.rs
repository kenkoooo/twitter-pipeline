use crate::current_time_duration;
use crate::sql::PgPoolExt;
use crate::twitter::TwitterClient;
use actix::clock::sleep;
use actix_web::rt::task::JoinHandle;
use anyhow::Result;
use egg_mode::user::follow;
use rand::prelude::*;
use sqlx::PgPool;
use std::collections::BTreeSet;
use std::iter::FromIterator;
use std::time::Duration;

pub struct FollowBackWorker {
    pub pool: PgPool,
    pub client: TwitterClient,
}

impl FollowBackWorker {
    pub fn start(self) -> JoinHandle<()> {
        actix::spawn(async move {
            let mut rng = thread_rng();
            loop {
                if let Err(e) = extract_and_follow(&self.pool, &self.client, &mut rng).await {
                    log::error!("{:?}", e);

                    log::info!("Sleeping 10 minutes ...");
                    sleep(Duration::from_secs(600)).await;
                }
            }
        })
    }
}

async fn extract_and_follow<R: Rng, P: PgPoolExt>(
    pool: &P,
    client: &TwitterClient,
    rng: &mut R,
) -> Result<()> {
    let one_hour_ago = current_time_duration().as_secs() - 3600;

    log::info!("Loading data ...");
    let friends_ids = pool.get_user_ids(one_hour_ago as i64, true).await?;
    let friends_set = BTreeSet::from_iter(friends_ids.into_iter());
    let followers_ids = pool.get_user_ids(one_hour_ago as i64, false).await?;
    let mut should_follow = followers_ids
        .into_iter()
        .filter(|user_id| !friends_set.contains(user_id))
        .collect::<Vec<_>>();
    should_follow.shuffle(rng);

    let mut no_data_user_ids = vec![];
    let mut user_data = vec![];
    for user_id in should_follow {
        if let Some(user) = pool.get_user_info(user_id).await? {
            user_data.push(user);
        } else {
            no_data_user_ids.push(user_id as u64);
            if no_data_user_ids.len() == 100 {
                break;
            }
        }
    }

    if !no_data_user_ids.is_empty() {
        log::info!("Fetching {} users", no_data_user_ids.len());
        let fetched_data = client.get_user_data(&no_data_user_ids).await?;
        for user_data in fetched_data.iter() {
            pool.put_user_info(user_data).await?;
        }
        user_data.extend(fetched_data);
    }

    log::info!("Following {} users", user_data.len());
    for user in user_data {
        log::info!("Following @{} ...", user.screen_name);
        follow(user.id, false, &client.token).await?;

        log::info!("Sleeping 10 minutes ...");
        sleep(Duration::from_secs(600)).await;
    }

    Ok(())
}
