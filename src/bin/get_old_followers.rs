use anyhow::Result;
use sqlx::PgPool;
use std::collections::BTreeMap;
use twitter_pipeline::current_time_duration;
use twitter_pipeline::sql::{PgPoolExt, UserIdClient};

#[actix_web::main]
async fn main() -> Result<()> {
    dotenv::dotenv()?;
    env_logger::init();
    let sql_url = std::env::var("SQL_URL")?;
    let pool = PgPool::connect(&sql_url).await?;

    let followers = pool.get_all_user_id_entries(true).await?;
    let friends = pool.get_all_user_id_entries(false).await?;

    let mut follower_confirmed_map = BTreeMap::new();
    for follower in followers {
        follower_confirmed_map.insert(follower.id, follower.confirmed_at);
    }

    let current_time_second = current_time_duration().as_secs() as i64;
    for friend in friends {
        if friend.confirmed_at < current_time_second - 3600 {
            continue;
        }

        let follow_confirmed = match follower_confirmed_map.get(&friend.id) {
            Some(&t) => t,
            None => continue,
        };

        let delta_second = friend.confirmed_at - follow_confirmed;
        if delta_second > 3600 {
            let user_info = pool.get_user_info(friend.id).await?;
            if let Some(user_info) = user_info {
                log::info!("https://twitter.com/{}", user_info.screen_name);
            }
        }
    }

    Ok(())
}
