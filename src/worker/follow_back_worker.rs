use crate::current_time_duration;
use crate::sql::PgPoolExt;
use crate::twitter::{RelationLookupExt, TwitterClient};
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
        }
    }

    log::info!("Fetching {} user data", no_data_user_ids.len());
    for user_ids in no_data_user_ids.chunks(100) {
        let fetched_data = client.get_user_data(&user_ids).await?;
        for user_data in fetched_data.iter() {
            pool.put_user_info(user_data).await?;
        }
        user_data.extend(fetched_data);

        log::info!("Sleeping 10 seconds ...");
        sleep(Duration::from_secs(10)).await;
    }

    let mut should_follow = vec![];
    for users in user_data.chunks(100) {
        let ids = users.iter().map(|user| user.id).collect::<Vec<_>>();
        let relations = client.get_relations(&ids).await?;
        for relation in relations {
            if relation.is_follower() && !relation.is_friend() && !relation.is_pending() {
                should_follow.push(relation);
            }
        }
        log::info!("Sleeping 3 minutes ...");
        sleep(Duration::from_secs(180)).await;
    }

    log::info!("Following {} users", should_follow.len());
    for user in should_follow {
        log::info!("Following @{} ...", user.screen_name);
        let response = follow(user.id, false, &client.token).await?;
        log::info!("Followed @{} ...", response.screen_name);

        log::info!("Sleeping 1 minutes ...");
        sleep(Duration::from_secs(60)).await;
    }

    Ok(())
}
