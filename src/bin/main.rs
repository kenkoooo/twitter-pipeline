use actix_web::{App, HttpServer};
use anyhow::Result;
use rand::thread_rng;
use sqlx::PgPool;
use std::io::stdin;
use twitter_pipeline::server::{get_remove_candidates, remove_user};
use twitter_pipeline::twitter::TwitterClient;
use twitter_pipeline::worker::InvalidUserRemover;
use twitter_pipeline::worker::UserIdSynchronizer;
use twitter_pipeline::worker::{FollowBackWorker, UserDataSynchronizer};

#[actix_web::main]
async fn main() -> Result<()> {
    dotenv::dotenv()?;
    env_logger::init();
    let sql_url = std::env::var("SQL_URL")?;
    let pool = PgPool::connect(&sql_url).await?;

    let consumer_key = std::env::var("CONSUMER_KEY")?;
    let consumer_secret = std::env::var("CONSUMER_SECRET")?;
    let token = egg_mode::KeyPair::new(consumer_key, consumer_secret);
    let request_token = egg_mode::auth::request_token(&token, "oob").await?;
    let url = egg_mode::auth::authorize_url(&request_token);
    log::info!("URL: {}", url);

    let input = read_input_line()?;
    let (token, _, screen_name) =
        egg_mode::auth::access_token(token, &request_token, input.trim()).await?;
    let client = TwitterClient { token, screen_name };

    let followers_ids_syncer = UserIdSynchronizer {
        pool: pool.clone(),
        client: client.clone(),
        follower: true,
    };
    let friends_ids_syncer = UserIdSynchronizer {
        pool: pool.clone(),
        client: client.clone(),
        follower: false,
    };
    let follow_back_worker = FollowBackWorker {
        pool: pool.clone(),
        client: client.clone(),
    };
    let invalid_user_remover = InvalidUserRemover {
        pool: pool.clone(),
        client: client.clone(),
    };
    let user_data_syncer = UserDataSynchronizer {
        pool: pool.clone(),
        client: client.clone(),
        rng: thread_rng(),
    };

    followers_ids_syncer.run();
    friends_ids_syncer.run();
    follow_back_worker.start();
    invalid_user_remover.start();
    user_data_syncer.start();

    HttpServer::new(move || {
        App::new()
            .service(get_remove_candidates)
            .service(remove_user)
            .data(client.clone())
            .data(pool.clone())
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await?;
    Ok(())
}

fn read_input_line() -> Result<String> {
    let mut buf = String::new();
    stdin().read_line(&mut buf)?;
    Ok(buf)
}
