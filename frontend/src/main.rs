use leptos::mount::mount_to_body;
use leptos::prelude::*;

use gloo_net::websocket::{Message, futures::WebSocket};
use futures::StreamExt;
use futures::channel::mpsc;
use leptos::task::spawn_local;

use shared::*;

#[component]
fn App() -> impl IntoView {
    let mut ws = WebSocket::open("ws://localhost:8787/room/test/ws").unwrap();
    let  (write, read) = ws.split();
    let (tx, rx) = mpsc::unbounded::<Message>();

    spawn_local(async move {
        let _ = rx.map(Ok).forward(write).await;
    });
    
    let (user, set_user) = signal(String::new());
    let (draft, set_draft) = signal(String::new());

    let send = move |_| {
        let user = user.get();
        let text = draft.get();

        let json = serde_json::to_string(&ClientMsg::Send { username: user, body: text }).unwrap();
        let _ = tx.unbounded_send(Message::Text(json));

        set_draft.set(String::new());
    };

    view! {
        <div class="composer">
            <input
                placeholder="User"
                prop:value=user
                on:input=move |ev| set_user.set(event_target_value(&ev))
            />
            <input
                placeholder="Say something"
                prop:value=draft
                on:input=move |ev| set_draft.set(event_target_value(&ev))
            />
            <button on:click=send>"Send"</button>
        </div>
    }
}

fn main() {
    mount_to_body(App);
}