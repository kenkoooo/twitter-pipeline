use crate::current_time_duration;
use crate::sql::PgPoolExt;
use crate::twitter::{RelationLookupExt, TwitterClient};
use actix_web::web::{Data, Json};
use actix_web::{get, post, HttpResponse, ResponseError};
use anyhow::Error;
use egg_mode::user::unfollow;
use rand::prelude::*;
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Display, Formatter};
use std::iter::FromIterator;

#[derive(Debug)]
pub struct ActixError(Error);
impl Display for ActixError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{:?}", self.0)
    }
}
impl ResponseError for ActixError {}
impl From<Error> for ActixError {
    fn from(e: Error) -> Self {
        Self(e)
    }
}

#[get("/remove_candidates")]
pub async fn get_remove_candidates(
    pool: Data<PgPool>,
    client: Data<TwitterClient>,
) -> Result<HttpResponse, ActixError> {
    let one_hour_ago = current_time_duration().as_secs() - 3600;
    let mut rng = StdRng::seed_from_u64(one_hour_ago);

    let friends_ids = pool.get_user_ids(one_hour_ago as i64, true).await?;
    let followers_ids = pool.get_user_ids(one_hour_ago as i64, false).await?;

    let followers_id_set = BTreeSet::from_iter(followers_ids.into_iter());
    let mut remove_candidate_ids = friends_ids
        .into_iter()
        .filter(|user_id| !followers_id_set.contains(user_id))
        .collect::<Vec<_>>();
    remove_candidate_ids.shuffle(&mut rng);

    let mut no_data_user = vec![];
    let mut user_data = vec![];
    for user_id in remove_candidate_ids {
        if let Some(user) = pool.get_user_info(user_id).await? {
            user_data.push(user);
        } else {
            no_data_user.push(user_id as u64);
            if no_data_user.len() == 100 {
                break;
            }
        }
    }

    if !no_data_user.is_empty() {
        log::info!("Fetching {} users", no_data_user.len());
        let fetched_data = client.get_user_data(&no_data_user).await?;
        for user_data in fetched_data.iter() {
            pool.put_user_info(user_data).await?;
        }
        user_data.extend(fetched_data);
    }

    user_data.shuffle(&mut rng);
    user_data.truncate(100);
    let user_ids = user_data.iter().map(|user| user.id).collect::<Vec<_>>();
    let relations = client.get_relations(&user_ids).await?;
    let mut relation_map = BTreeMap::new();
    for relation in relations {
        relation_map.insert(relation.id, relation);
    }

    let user_data = user_data
        .into_iter()
        .filter(|user| {
            if let Some(relation) = relation_map.get(&user.id) {
                relation.is_friend() && !relation.is_pending() && !relation.is_follower()
            } else {
                false
            }
        })
        .collect::<Vec<_>>();

    Ok(HttpResponse::Ok().json(user_data))
}

#[derive(Serialize, Deserialize)]
pub struct RemoveRequest {
    user_id: i64,
}

#[post("/remove_user")]
pub async fn remove_user(
    request: Json<RemoveRequest>,
    client: Data<TwitterClient>,
) -> Result<HttpResponse, ActixError> {
    log::info!("Removing {}", request.user_id);
    let result = unfollow(request.user_id as u64, &client.token)
        .await
        .map_err(|e| anyhow::Error::from(e))?;
    log::info!("Removed @{}", result.response.screen_name);
    Ok(HttpResponse::Ok().json(result.response))
}
