use crate::sql::queue::{ConfirmationQueue, Message, MessageQueue, MessageType};
use crate::sql::PgPoolExt;
use crate::twitter::TwitterClient;
use actix_web::web::{Data, Json};
use actix_web::{get, post, HttpResponse, ResponseError};
use anyhow::Error;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::fmt::{Display, Formatter};

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
    let mut user_ids = vec![];
    while let Some(user_id) = pool.pop_remove_candidate().await? {
        if pool.is_in_white_list(user_id as i64).await? {
            continue;
        }
        user_ids.push(user_id);
        if user_ids.len() == 100 {
            break;
        }
    }

    let user_data = client.get_user_data(&user_ids).await?;
    for user_data in user_data.iter() {
        pool.put_user_info(user_data).await?;
    }

    Ok(HttpResponse::Ok().json(user_data))
}

#[derive(Serialize, Deserialize)]
pub struct AllowUserRequest {
    user_id: i64,
}

#[post("/allow_user")]
pub async fn post_allow_user(
    request: Json<AllowUserRequest>,
    pool: Data<PgPool>,
) -> Result<HttpResponse, ActixError> {
    log::info!("Allowing {}", request.user_id);
    pool.put_white_list(request.user_id).await?;
    Ok(HttpResponse::Ok().json(request.user_id))
}

#[derive(Serialize, Deserialize)]
pub struct ConfirmRemoveRequest {
    user_id: i64,
}

#[post("/confirm_remove")]
pub async fn post_confirm_remove(
    request: Json<ConfirmRemoveRequest>,
    pool: Data<PgPool>,
) -> Result<HttpResponse, ActixError> {
    log::info!("Confirm removing {}", request.user_id);
    pool.push_message(Message {
        user_id: request.user_id as u64,
        message_type: MessageType::Remove,
    })
    .await?;
    Ok(HttpResponse::Ok().json(request.user_id))
}
