mod chat;
mod file;
mod message;
mod user;
mod workspace;

use serde::{Deserialize, Serialize};

pub use chat::CreateChat;
pub use message::*;
pub use user::{CreateUser, SigninUser};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatFile {
    pub ws_id: u64,
    pub hash: String,
    pub ext: String, // extract ext from filename or mime type
}
