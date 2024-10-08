use crate::{AppError, AppState};

use chat_core::WorkSpace;

impl AppState {
    pub async fn create_workspace(&self, name: &str, user_id: u64) -> Result<WorkSpace, AppError> {
        let ws = sqlx::query_as(
            r#"
            INSERT INTO workspaces (name, owner_id)
            VALUES ($1, $2)
            RETURNING id, name, owner_id, created_at
            "#,
        )
        .bind(name)
        .bind(user_id as i64)
        .fetch_one(&self.pool)
        .await?;
        Ok(ws)
    }

    pub async fn find_workspace_by_name(&self, name: &str) -> Result<Option<WorkSpace>, AppError> {
        let ws = sqlx::query_as(
            r#"
            SELECT id, name, owner_id, created_at
            FROM workspaces
            WHERE name = $1
            "#,
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;
        Ok(ws)
    }

    #[allow(unused)]
    pub async fn find_workspace_by_id(&self, id: u64) -> Result<Option<WorkSpace>, AppError> {
        let ws = sqlx::query_as(
            r#"
            SELECT id, name, owner_id, created_at
            FROM workspaces
            WHERE id = $1
            "#,
        )
        .bind(id as i64)
        .fetch_optional(&self.pool)
        .await?;
        Ok(ws)
    }

    pub async fn update_workspace_owner(
        &self,
        id: u64,
        owner_id: u64,
    ) -> Result<WorkSpace, AppError> {
        // update owner_id in two cases 1) owner_id = 0 3) owner's ws_id = id
        let ws = sqlx::query_as(
            r#"
            UPDATE workspaces
            SET owner_id = $1
            WHERE id = $2 and (SELECT ws_id FROM users WHERE id = $1) = $2
            RETURNING id, name, owner_id, created_at
            "#,
        )
        .bind(owner_id as i64)
        .bind(id as i64)
        .fetch_one(&self.pool)
        .await?;
        Ok(ws)
    }
}
#[cfg(test)]
mod tests {
    use anyhow::{Ok, Result};

    use super::*;
    use crate::models::CreateUser;

    #[tokio::test]
    async fn workspace_should_create_and_set_owner() -> Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;
        let ws = state.create_workspace("test", 0).await.unwrap();
        let input = CreateUser::new(&ws.name, "hp1", "hp@none.com", "123");
        let user = state.create_user(&input).await.unwrap();

        assert_eq!(ws.name, "test");
        assert_eq!(user.ws_id, ws.id);

        let ws = state
            .update_workspace_owner(ws.id as _, user.id as u64)
            .await
            .unwrap();
        assert_eq!(ws.owner_id, user.id);
        Ok(())
    }

    #[tokio::test]
    async fn workspace_should_find_by_name() -> Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;
        let ws = state.find_workspace_by_name("acme").await?;

        assert_eq!(ws.unwrap().name, "acme");
        Ok(())
    }

    #[tokio::test]
    async fn workspace_should_fetch_all_chat_users() -> Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;

        let users = state.fetch_chat_users(1).await?;
        assert_eq!(users.len(), 5);
        assert_eq!(users[0].fullname, "hp");
        assert_eq!(users[1].fullname, "zsr");
        Ok(())
    }
}
