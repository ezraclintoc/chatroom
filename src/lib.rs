use worker::*;
use worker::{durable_object, DurableObject, State, Env, Result, Request, Response, SqlStorage, WebSocket, WebSocketPair, WebSocketIncomingMessage};

#[durable_object]
pub struct Chatroom {
    sql: SqlStorage,
    state: State,
}

impl DurableObject for Chatroom {
    fn new(state: State, _env: Env) -> Self {
        let sql = state.storage().sql();
        sql.exec("CREATE TABLE IF NOT EXISTS chatroom(id INTEGER PRIMARY KEY, user TEXT NOT NULL, nody TEXT NOT NULL, sent_at INTEGER NOT NULL)", None).expect("Create Table");
        Self {
            sql,
            state,
        }
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
            WebSocketIncomingMessage::String(text) => { for w in self.state.get_websockets() { let _ = w.send_with_str(&text); } },
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
async fn fetch(
    req: Request,
    env: Env,
    _ctx: Context,
) -> Result<Response> {
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