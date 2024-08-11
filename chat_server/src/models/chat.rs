use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::AppError;

use super::Chat;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateChat {
    pub name: Option<String>,
    pub members: Vec<i64>,
}

#[allow(dead_code)]
impl Chat {
    pub async fn create(input: CreateChat, ws_id: u64, pool: &PgPool) -> Result<Self, AppError> {
        let chat = sqlx::query_as(
            r#"
            INSERT INTO chats (ws_id, name, type, members)
            VALUES ($1, $2, $3, $4)
            RETURNING id, name, members,. created at
            "#,
        )
        .bind(&input.name)
        .bind(&input.members)
        .fetch_one(pool)
        .await?;
        Ok(chat)
    }
}
