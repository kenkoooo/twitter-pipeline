use crate::sql::PgPoolExt;
use crate::twitter::{RelationLookupExt, TwitterClient};
use crate::{current_time_duration, get_difference};
use actix::clock::sleep;
use actix_web::rt::task::JoinHandle;
use anyhow::Result;
use egg_mode::user::follow;
use rand::prelude::*;
use sqlx::PgPool;
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
    let mut should_follow = get_difference(pool, one_hour_ago as i64, true).await?;
    should_follow.shuffle(rng);

    let mut confirmed_users = vec![];
    for user_id in should_follow.chunks(100) {
        let ids = user_id.into_iter().map(|&x| x as u64).collect::<Vec<_>>();
        let relations = client.get_relations(&ids, true).await?;
        for relation in relations {
            if relation.is_follower() && !relation.is_friend() && !relation.is_pending() {
                confirmed_users.push(relation);
            }
        }
    }

    log::info!("Following {} users", confirmed_users.len());
    for relation in confirmed_users {
        log::info!("Following @{} ...", relation.screen_name);
        let response = follow(relation.id, false, &client.token).await?;
        log::info!("Followed @{} ...", response.screen_name);

        log::info!("Sleeping 1 minutes ...");
        sleep(Duration::from_secs(60)).await;
    }

    Ok(())
}
