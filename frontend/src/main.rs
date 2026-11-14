use std::time::{Duration, Instant};
use std::collections::BTreeMap;
use leptos::mount::mount_to_body;
use leptos::prelude::*;

use futures::StreamExt;
use futures::channel::mpsc;
use gloo_net::websocket::{Message, futures::WebSocket};
use leptos::task::spawn_local;

use shared::*;

#[component]
fn App() -> impl IntoView {
    let location = window().location();
    let host = if location.port().unwrap() == "8080" {
        "localhost:8787".to_string() // dev: Trunk on 8080, Wrangler on 8787
    } else {
        location.host().unwrap() //prod
    };
    let scheme = if location.protocol().unwrap() == "https:" { "wss" } else { "ws" };
    let url = format!("{scheme}://{host}/api/room/test/ws");
     leptos::logging::log!("got: {url}");
    let ws = WebSocket::open(&url).unwrap();
    let (write, mut read) = ws.split();
    let (tx, rx) = mpsc::unbounded::<Message>();

    let (user, set_user) = signal(String::new());
    let (draft, set_draft) = signal(String::new());

    let messages = RwSignal::new(Vec::<ChatMessage>::new());
    let typing = RwSignal::new(BTreeMap::<String, i64>::new());
    let error = RwSignal::new(None::<String>);

    set_interval(
        move || {
            let now = js_sys::Date::now() as i64;
            let expired = typing.with(|v| v.iter().any(|(_, t)| now - *t >= 4000));
            if expired {
                typing.update(|m| m.retain(|_name, t| now - *t < 4000));
            }
        },
        Duration::from_secs(1),
    );
    spawn_local(async move {
        let _ = rx.map(Ok).forward(write).await;
    });
    spawn_local(async move {
        while let Some(Ok(Message::Text(text))) = read.next().await {
            match serde_json::from_str::<ServerMsg>(&text) {
                Ok(ServerMsg::History { messages: history }) => messages.set(history),
                Ok(ServerMsg::Message(m)) => { typing.update(|u| u.retain(|name, _t| *name != m.username)); messages.update(|v| v.push(m)) },
                Ok(ServerMsg::Error { reason }) => error.set(Some(reason)),
                Ok(ServerMsg::Typing { username }) => typing.update(|u| { u.insert(username,  js_sys::Date::now() as i64); }),
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
                                <div class="content">
                                    <span class="username">{m.username}</span>
                                    " "
                                    <span class="body">{m.body}</span>
                                </div>
                                <span class="time">{format_time(m.sent_at)}</span>
                            </div>
                        }
                    />
                </div>
                <div class="typing">
                    {move || {
                        let names: Vec<String> = typing.with(|v| v.iter().map(|(n, _)| n.clone()).collect());
                        match names.len() {
                            0 => String::new(),
                            1 => format!("{} is typing...", names[0]),
                            2 => format!("{} and {} are typing...", names[0], names[1]),
                            _ => "Several people are typing...".to_string(),
                        }
                    }}
                </div>
                {move || error.get().map(|e| view! { <div class="error">{e}</div> })}
                <div class="composer">
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
                        on:input=move |ev| {
                            let username = user.get();
                            let json = serde_json::to_string(&ClientMsg::Typing { username }).unwrap();
                            tx.with_value(|tx| {
                                let _ = tx.unbounded_send(Message::Text(json));
                            });
                            set_draft.set(event_target_value(&ev))}
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

use wasm_bindgen::JsValue;

fn format_time(ms: i64) -> String {
    let date = js_sys::Date::new(&JsValue::from_f64(ms as f64));
    let opts = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&opts, &"hour".into(), &"numeric".into());
    let _ = js_sys::Reflect::set(&opts, &"minute".into(), &"2-digit".into());
    date.to_locale_time_string_with_options("en-US", &opts).into()
}

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(App);
}
