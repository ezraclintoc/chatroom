use serde::{Deserialize, Serialize};
use worker::*;
use worker::{
    durable_object, DurableObject, Env, Request, Response, Result, SqlStorage, State, WebSocket,
    WebSocketIncomingMessage, WebSocketPair,
};

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

#[durable_object]
pub struct Chatroom {
    sql: SqlStorage,
    state: State,
}

impl DurableObject for Chatroom {
    fn new(state: State, _env: Env) -> Self {
        let sql = state.storage().sql();
        sql.exec("CREATE TABLE IF NOT EXISTS chatroom(id INTEGER PRIMARY KEY, user TEXT NOT NULL, body TEXT NOT NULL, sent_at INTEGER NOT NULL)", None).expect("Create Table");
        Self { sql, state }
    }

    async fn fetch(&self, req: Request) -> Result<Response> {
        let upgrade = req.headers().get("Upgrade")?;
        if upgrade.as_deref() != Some("websocket") {
            return Response::error("Expected WebSocket upgrade", 426);
        }

        let pair = WebSocketPair::new()?;

        self.state.accept_web_socket(&pair.server);

        Response::from_websocket(pair.client)
    }

    async fn websocket_message(
        &self,
        ws: WebSocket,
        message: WebSocketIncomingMessage,
    ) -> Result<()> {
        // The let _ = may not send to all websockets
        match message {
            WebSocketIncomingMessage::String(text) => {
                match serde_json::from_str::<ClientMsg>(&text) {
                    Ok(ClientMsg::Send { username, body }) => {
                        let sent_at = Date::now().as_millis() as i64;

                        #[derive(serde::Deserialize)]
                        struct IdRow {
                            id: i64,
                        }
                        let rows: Vec<IdRow> = self.sql.exec(
                            "INSERT INTO chatroom(user, body, sent_at) VALUES (?, ?, ?) RETURNING id",
                            vec![username.clone().into(), body.clone().into(), sent_at.into()],
                        )?.to_array()?;

                        let id = match rows.first() {
                            Some(row) => row.id,
                            None => return Err(Error::RustError("insert returned no id".into())),
                        };

                        let msg = ServerMsg::Message(ChatMessage { id, username, body, sent_at });
                        let json = serde_json::to_string(&msg)?;
                        
                        for w in self.state.get_websockets() {
                            let _ = w.send_with_str(&json);
                        }
                    }
                    Err(_) => { }
                }
            }
            WebSocketIncomingMessage::Binary(_) => ws.send_with_str("binary not supported")?,
        }
        Ok(())
    }

    async fn websocket_close(
        &self,
        ws: WebSocket,
        code: usize,
        reason: String,
        _was_clean: bool,
    ) -> Result<()> {
        ws.close(Some(code as u16), Some(reason))?;
        Ok(())
    }
}

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    Router::new()
        .get_async("/room/:name/ws", |req, ctx| async move {
            let name = match ctx.param("name") {
                Some(n) => n.clone(),
                None => return Response::error("missing name", 400),
            };
            let namespace = ctx.durable_object("CHATROOM")?;
            let stub = namespace.id_from_name(&name)?.get_stub()?;
            stub.fetch_with_request(req).await
        })
        .run(req, env)
        .await
}
