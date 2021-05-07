use crate::sql::queue::{ConfirmationQueue, Message, MessageQueue, MessageType};
use crate::sql::PgPoolExt;
use crate::twitter::{RelationLookupExt, TwitterClient};
use actix_web::rt::task::JoinHandle;
use anyhow::Result;
use rand::prelude::*;
use sqlx::PgPool;
use std::collections::BTreeSet;
use std::iter::FromIterator;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub struct RelationSynchronizer<R> {
    pub client: TwitterClient,
    pub pool: PgPool,
    pub rng: R,
}

impl<R: Rng + 'static> RelationSynchronizer<R> {
    pub fn run(self) -> JoinHandle<()> {
        let pool = self.pool;
        let mut rng = self.rng;
        let client = self.client;
        actix::spawn(async move {
            loop {
                if let Err(e) = extract_and_push(&pool, &client, &mut rng).await {
                    log::error!("{:?}", e);
                }

                let duration = Duration::from_secs(60);
                log::info!("Sleep {} seconds ...", duration.as_secs());
                actix::clock::sleep(duration).await;
            }
        })
    }
}

async fn extract_and_push<R: Rng>(
    pool: &PgPool,
    client: &TwitterClient,
    rng: &mut R,
) -> Result<()> {
    let (mut unremoved, mut unfollowed) = get_candidate_ids(pool).await?;
    log::info!(
        "unremoved={} unfollowed={}",
        unremoved.len(),
        unfollowed.len()
    );

    unfollowed.shuffle(rng);
    unfollowed.truncate(100);
    let follow_candidates = client
        .get_relations(&unfollowed)
        .await?
        .into_iter()
        .filter(|r| r.is_follower() && !r.is_friend() && !r.is_pending())
        .map(|relation| relation.id)
        .collect::<Vec<_>>();
    for user_id in follow_candidates {
        let message = Message {
            user_id,
            message_type: MessageType::Follow,
        };
        pool.push_message(message).await?;
    }

    unremoved.shuffle(rng);
    unremoved.truncate(100);
    let remove_candidates = client
        .get_relations(&unremoved)
        .await?
        .into_iter()
        .filter(|r| !r.is_follower() && r.is_friend())
        .map(|relation| relation.id)
        .collect::<Vec<_>>();
    for user_id in remove_candidates {
        pool.push_remove_candidate(user_id).await?;
    }

    Ok(())
}

async fn get_candidate_ids(pool: &PgPool) -> Result<(Vec<u64>, Vec<u64>)> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?;
    let one_hour_before = now - Duration::from_secs(3600);
    let friends_ids = pool
        .get_user_ids(one_hour_before.as_secs() as i64, true)
        .await?;
    let friends_ids = BTreeSet::from_iter(friends_ids.into_iter().map(|id| id as u64));
    let followers_ids = pool
        .get_user_ids(one_hour_before.as_secs() as i64, false)
        .await?;
    let followers_ids = BTreeSet::from_iter(followers_ids.into_iter().map(|id| id as u64));

    let unremoved = friends_ids
        .difference(&followers_ids)
        .cloned()
        .collect::<Vec<_>>();
    let unfollowed = followers_ids
        .difference(&friends_ids)
        .cloned()
        .collect::<Vec<_>>();

    Ok((unremoved, unfollowed))
}
