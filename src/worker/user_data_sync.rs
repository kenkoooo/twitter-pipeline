use crate::current_time_duration;
use crate::sql::PgPoolExt;
use crate::twitter::TwitterClient;
use actix::clock::sleep;
use actix_web::rt::task::JoinHandle;
use anyhow::Result;
use rand::prelude::*;
use std::time::Duration;

pub struct UserDataSynchronizer<P, R> {
    pub pool: P,
    pub client: TwitterClient,
    pub rng: R,
}

impl<P: PgPoolExt + 'static, R: Rng + 'static> UserDataSynchronizer<P, R> {
    pub fn start(self) -> JoinHandle<()> {
        actix::spawn(async move {
            let mut rng = self.rng;
            loop {
                if let Err(e) = fetch_user_data(&self.pool, &self.client, &mut rng).await {
                    log::error!("{:?}", e);
                    log::info!("Sleeping 5 minutes");
                    sleep(Duration::from_secs(300)).await;
                } else {
                    log::info!("Sleeping 10 seconds");
                    sleep(Duration::from_secs(10)).await;
                }
            }
        })
    }
}

async fn fetch_user_data<P: PgPoolExt, R: Rng>(
    pool: &P,
    client: &TwitterClient,
    rng: &mut R,
) -> Result<()> {
    let one_hour_ago = current_time_duration().as_secs() - 3600;
    let mut user_ids = pool.get_no_data_user_ids(one_hour_ago as i64, 1000).await?;
    user_ids.shuffle(rng);

    if user_ids.len() > 100 {
        user_ids.truncate(100);
    } else {
        user_ids.truncate(1);
    }
    if !user_ids.is_empty() {
        let user_ids = user_ids.into_iter().map(|i| i as u64).collect::<Vec<_>>();
        let user_data = client.get_user_data(&user_ids, true).await?;
        for user_data in user_data {
            pool.put_user_info(&user_data).await?;
        }
    }
    Ok(())
}
