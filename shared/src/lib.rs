use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub id: i64,
    pub username: String,
    pub body: String,
    pub sent_at: i64,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMsg {
    Send { username: String, body: String },
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMsg {
    Message(ChatMessage),
    History { messages: Vec<ChatMessage> },
    Error { reason: String },
}
