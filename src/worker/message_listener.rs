use crate::sql::queue::{Message, MessageQueue, MessageType};
use crate::twitter::TwitterClient;
use actix::clock::sleep;
use actix_web::rt::task::JoinHandle;
use egg_mode::user::follow;
use sqlx::PgPool;
use std::time::Duration;

pub struct MessageListener {
    pub pool: PgPool,
    pub client: TwitterClient,
}

impl MessageListener {
    pub fn run(self) -> JoinHandle<()> {
        actix::spawn(async move {
            loop {
                let message = self.pool.pop_message().await;
                match message {
                    Ok(Some(Message {
                        message_type,
                        user_id,
                    })) => match message_type {
                        MessageType::Follow => {
                            match follow(user_id, false, &self.client.token).await {
                                Ok(response) => {
                                    let screen_name = response.response.screen_name;
                                    log::info!("Followed {}", screen_name);
                                }
                                Err(e) => {
                                    log::info!("{:?}", e);
                                }
                            }
                        }
                        MessageType::Remove => {
                            log::warn!("Removing is unimplemented!");
                        }
                    },
                    Ok(None) => {
                        log::info!("message_queue is empty.");
                    }
                    Err(e) => {
                        log::error!("{:?}", e);
                    }
                }

                let duration = Duration::from_secs(60);
                log::info!("Sleep {} seconds", duration.as_secs());
                sleep(duration).await;
            }
        })
    }
}
