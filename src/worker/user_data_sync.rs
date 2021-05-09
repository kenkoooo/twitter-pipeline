use crate::sql::PgPoolExt;
use crate::twitter::TwitterClient;
use actix::clock::sleep;
use actix_web::rt::task::JoinHandle;
use anyhow::Result;
use std::time::Duration;

pub struct UserDataSynchronizer<P> {
    pub pool: P,
    pub client: TwitterClient,
}

impl<P: PgPoolExt + 'static> UserDataSynchronizer<P> {
    pub fn start(self) -> JoinHandle<()> {
        actix::spawn(async move {
            loop {
                if let Err(e) = fetch_user_data(&self.pool, &self.client).await {
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

async fn fetch_user_data<P: PgPoolExt>(pool: &P, client: &TwitterClient) -> Result<()> {
    let user_ids = pool.get_no_data_user_ids().await?;
    let user_data = client
        .get_user_data(&user_ids.into_iter().map(|i| i as u64).collect::<Vec<_>>())
        .await?;
    for user_data in user_data {
        pool.put_user_info(&user_data).await?;
    }
    Ok(())
}
