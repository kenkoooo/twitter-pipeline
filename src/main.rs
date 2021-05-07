use actix_web::{get, web, App, HttpServer, Responder};
use anyhow::Result;
use rand::prelude::StdRng;
use rand::SeedableRng;
use serde::Deserialize;
use sqlx::PgPool;
use std::io::stdin;
use twitter_pipeline::twitter::TwitterClient;
use twitter_pipeline::worker::message_listener::MessageListener;
use twitter_pipeline::worker::relation_sync::RelationSynchronizer;
use twitter_pipeline::worker::user_id_sync::UserIdSynchronizer;

#[derive(Deserialize)]
struct Info {
    id: u32,
    name: String,
}
#[get("/{id}/{name}/index.html")]
async fn index(path: web::Path<Info>) -> impl Responder {
    format!("Hello {}! id:{}", path.name, path.id)
}

#[actix_web::main]
async fn main() -> Result<()> {
    dotenv::dotenv()?;
    env_logger::init();

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

    let sql_url = std::env::var("SQL_URL")?;
    let pool = PgPool::connect(&sql_url).await?;

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
    let relation_syncer = RelationSynchronizer {
        pool: pool.clone(),
        client: client.clone(),
        rng: StdRng::seed_from_u64(717),
    };
    let message_listener = MessageListener {
        pool: pool.clone(),
        client: client.clone(),
    };

    followers_ids_syncer.run();
    friends_ids_syncer.run();
    relation_syncer.run();
    message_listener.run();

    HttpServer::new(move || App::new().service(index).data(pool.clone()))
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
