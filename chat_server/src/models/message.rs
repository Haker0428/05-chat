use std::str::FromStr;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{AppError, AppState, ChatFile};

use super::Message;

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMessage {
    pub content: String,
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListMessage {
    pub last_id: Option<u64>,
    pub limit: u64,
}

#[allow(dead_code)]
impl AppState {
    pub async fn create_message(
        &self,
        input: CreateMessage,
        chat_id: u64,
        user_id: u64,
    ) -> Result<Message, AppError> {
        let base_dir = &self.config.server.base_dir;
        // verify content - not empty
        if input.content.is_empty() {
            return Err(AppError::CreateMessageError(
                "Content cannot be empty".to_string(),
            ));
        }

        // verify files exist
        for s in &input.files {
            println!("s: {}", s);
            let file = ChatFile::from_str(s)?;
            if !file.path(&base_dir).exists() {
                return Err(AppError::CreateMessageError(format!(
                    "File {} does not exist",
                    s
                )));
            }
        }

        let messagge: Message = sqlx::query_as(
            r#"
            INSERT INTO messages (chat_id, sender_id, content, files)
            VALUES ($1, $2, $3, $4)
            RETURNING id, chat_id, sender_id, content, files, created_at
            "#,
        )
        .bind(chat_id as i64)
        .bind(user_id as i64)
        .bind(input.content)
        .bind(&input.files)
        .fetch_one(&self.pool)
        .await?;

        Ok(messagge)
    }

    pub async fn list_messages(
        &self,
        input: ListMessage,
        chat_id: u64,
    ) -> Result<Vec<Message>, AppError> {
        let last_id = input.last_id.unwrap_or(i64::MAX as _);

        let messages: Vec<Message> = sqlx::query_as(
            r#"
        SELECT id, chat_id, sender_id, content, files, created_at
        FROM messages
        WHERE chat_id = $1
        AND id < $2
        ORDER BY id DESC
        LIMIT $3
        "#,
        )
        .bind(chat_id as i64)
        .bind(last_id as i64)
        .bind(input.limit as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(messages)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[tokio::test]
    async fn create_message_should_work() -> Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;
        let input = CreateMessage {
            content: "Hello".to_string(),
            files: vec![],
        };

        let message = state
            .create_message(input, 1, 1)
            .await
            .expect("create message failed");

        // verify the message content
        assert_eq!(message.content, "Hello");

        // invalid files should fail
        let input = CreateMessage {
            content: "Hello".to_string(),
            files: vec!["1".to_string()],
        };

        let err = state.create_message(input, 1, 1).await.unwrap_err();
        assert_eq!(err.to_string(), "Invalid file path: 1");

        // valid files should work
        let url = upload_dummy_file(&state)?;
        println!("url: {}", url);
        let input = CreateMessage {
            content: "Hello".to_string(),
            files: vec![url],
        };

        let message = state
            .create_message(input, 1, 1)
            .await
            .expect("create message failed");
        assert_eq!(message.content, "Hello");
        assert_eq!(message.files.len(), 1);

        Ok(())
    }

    #[tokio::test]
    async fn list_message_should_work() -> Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;
        let input = ListMessage {
            last_id: None,
            limit: 6,
        };

        let messages = state
            .list_messages(input, 1)
            .await
            .expect("list messages failed");

        assert_eq!(messages.len(), 6);

        let last_id = messages.last().expect("last message should exists").id;
        let input = ListMessage {
            last_id: Some(last_id as _),
            limit: 6,
        };
        let messages = state.list_messages(input, 1).await?;
        assert_eq!(messages.len(), 4);
        Ok(())
    }

    fn upload_dummy_file(state: &AppState) -> Result<String> {
        let file = ChatFile::new(1, "test.txt", b"hello");
        let path = file.path(&state.config.server.base_dir);
        std::fs::create_dir_all(path.parent().expect("file path parent should exists"))?;
        std::fs::write(&path, b"hello word")?;
        Ok(file.url())
    }
}
