use leptos::mount::mount_to_body;
use leptos::prelude::*;

use futures::StreamExt;
use futures::channel::mpsc;
use gloo_net::websocket::{Message, futures::WebSocket};
use leptos::task::spawn_local;

use shared::*;

#[component]
fn App() -> impl IntoView {
    let mut ws = WebSocket::open("ws://localhost:8787/room/test/ws").unwrap();
    let (write, mut read) = ws.split();
    let (tx, rx) = mpsc::unbounded::<Message>();

    let (user, set_user) = signal(String::new());
    let (draft, set_draft) = signal(String::new());

    let messages = RwSignal::new(Vec::<ChatMessage>::new());
    let error = RwSignal::new(None::<String>);

    spawn_local(async move {
        let _ = rx.map(Ok).forward(write).await;
    });
    spawn_local(async move {
        while let Some(Ok(Message::Text(text))) = read.next().await {
            match serde_json::from_str::<ServerMsg>(&text) {
                Ok(ServerMsg::History { messages: history }) => messages.set(history),
                Ok(ServerMsg::Message(m)) => messages.update(|v| v.push(m)),
                Ok(ServerMsg::Error { reason }) => error.set(Some(reason)),
                Err(e) => leptos::logging::error!("bad server message: {e}"),
            }
        }
    });

    let tx = StoredValue::new(tx);
    
    let send = move || {
        let username = user.get();
        let body = draft.get();
        let json = serde_json::to_string(&ClientMsg::Send { username, body }).unwrap();
        tx.with_value(|tx| {
            let _ = tx.unbounded_send(Message::Text(json));
        });
        set_draft.set(String::new());
    };
    
    let body_ref = NodeRef::<leptos::html::Input>::new();

    view! {
        <div class="app">
            <div class="chat">
                <div class="messages">
                    <For
                        each=move || messages.get()
                        key=|m| m.id
                        children=move |m| view! {
                            <div class="message">
                                <div class="meta">
                                    <span class="username">{m.username}</span>
                                </div>
                                <div class="body">{m.body}</div>
                            </div>
                        }
                    />
                </div>
                {move || error.get().map(|e| view! { <div class="error">{e}</div> })}
                <<div class="composer">
                    <input
                        placeholder="User"
                        prop:value=user
                        on:input=move |ev| set_user.set(event_target_value(&ev))
                        on:keydown=move |ev| {
                            if ev.key() == "Enter" {
                                if let Some(input) = body_ref.get() {
                                    let _ = input.focus();
                                }
                            }
                        }
                    />
                    <input
                        placeholder="Say something"
                        node_ref=body_ref
                        prop:value=draft
                        on:input=move |ev| set_draft.set(event_target_value(&ev))
                        on:keydown=move |ev| {
                            if ev.key() == "Enter" {
                                send();
                            }
                        }
                    />
                    <button on:click=move |_| send()>"Send"</button>
                </div>
            </div>
        </div>
    }
}

fn main() {
    mount_to_body(App);
}
