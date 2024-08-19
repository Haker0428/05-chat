use std::{collections::HashSet, sync::Arc};

use chat_core::{Chat, Message};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgListener;
use tokio_stream::StreamExt;
use tracing::{info, warn};

use crate::AppState;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AppEvent {
    NewChat(Chat),
    AddToChat(Chat),
    RemoveFromChat(Chat),
    NewMessage(Message),
}

#[derive(Debug)]
pub struct Notification {
    // users being notified, so we should notify them
    user_ids: HashSet<u64>,
    event: Arc<AppEvent>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ChatUpdated {
    op: String,
    old: Option<Chat>,
    new: Option<Chat>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ChatMessageCreated {
    chat: Chat,
    message: Message,
}

pub async fn setup_pg_listener(state: AppState) -> anyhow::Result<()> {
    let mut listener = PgListener::connect(&state.config.server.db_url).await?;
    listener.listen("chat_updated").await?;
    listener.listen("chat_message_created").await?;

    let mut stream = listener.into_stream();

    tokio::spawn(async move {
        while let Some(Ok(notif)) = stream.next().await {
            info!("notification: {:?}", notif);
            let notification = Notification::load(notif.channel(), notif.payload())?;
            let _users = &state.users;
            for user_id in notification.user_ids {
                if let Some(tx) = state.users.get(&user_id) {
                    info!("sending notification to user {}", user_id);
                    if let Err(e) = tx.send(notification.event.clone()) {
                        warn!("Failed to send notification to user {}: {}", user_id, e);
                    }
                }
            }
        }

        Ok::<_, anyhow::Error>(())
    });

    Ok(())
}

impl Notification {
    fn load(r#type: &str, paylod: &str) -> anyhow::Result<Self> {
        match r#type {
            "chat_updated" => {
                let payload: ChatUpdated = serde_json::from_str(paylod)?;
                info!("chat_updated. payload: {:?}", payload);
                let user_ids =
                    get_affected_cbat_user_ids(payload.old.as_ref(), payload.new.as_ref());
                let event = match payload.op.as_str() {
                    "INSERT" => AppEvent::NewChat(payload.new.expect("new should exist")),
                    "UPDATE" => AppEvent::AddToChat(payload.new.expect("new should exist")),
                    "DELETE" => AppEvent::RemoveFromChat(payload.old.expect("old should exist")),
                    _ => return Err(anyhow::anyhow!("Invalid op: {}", payload.op)),
                };
                Ok(Self {
                    user_ids,
                    event: Arc::new(event),
                })
            }
            "chat_message_created" => {
                let payload: ChatMessageCreated = serde_json::from_str(paylod)?;
                info!("chat_message_created. payload: {:?}", payload);
                let user_ids = payload.chat.members.iter().map(|v| *v as u64).collect();
                Ok(Self {
                    user_ids,
                    event: Arc::new(AppEvent::NewMessage(payload.message)),
                })
            }
            _ => {
                info!("Invalid type: {}", r#type);
                Err(anyhow::anyhow!("Invalid type: {}", r#type))
            }
        }
    }
}

fn get_affected_cbat_user_ids(old: Option<&Chat>, new: Option<&Chat>) -> HashSet<u64> {
    match (old, new) {
        (Some(old), Some(new)) => {
            // diff old/new members, if identical, no need to notify, otherwise notify the union of both
            let old_members: HashSet<_> = old.members.iter().map(|v| *v as u64).collect();
            let new_members: HashSet<_> = new.members.iter().map(|v| *v as u64).collect();
            if old_members == new_members {
                HashSet::new()
            } else {
                old_members.union(&new_members).copied().collect()
            }
        }
        (Some(old), None) => old.members.iter().map(|v| *v as u64).collect(),
        (None, Some(new)) => new.members.iter().map(|v| *v as u64).collect(),
        (None, None) => HashSet::new(),
    }
}
