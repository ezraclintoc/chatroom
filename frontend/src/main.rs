use std::time::Duration;
use std::collections::BTreeMap;
use leptos_router::components::{Route, Router, Routes};
use leptos_router::hooks::{use_navigate, use_params_map};
use leptos_router::path;
use leptos::prelude::*;
use leptos::either::Either;
use leptos_router::components::Redirect;

use futures::StreamExt;
use futures::channel::mpsc;
use gloo_net::websocket::{Message, futures::WebSocket};
use leptos::task::spawn_local;

use shared::*;

#[component]
fn Room() -> impl IntoView {
    if load_username().trim().is_empty() {
        Either::Left(view! { <Redirect path="/" /> })
    } else {
        Either::Right(view! { <ChatRoom /> })
    }
}

#[component]
fn ChatRoom() -> impl IntoView {
    let params = use_params_map();
    let room_name = params.read_untracked().get("name").unwrap_or_default();
    
    let location = window().location();
    let host = if location.port().unwrap() == "8080" {
        "localhost:8787".to_string() // dev: Trunk on 8080, Wrangler on 8787
    } else {
        location.host().unwrap() //prod
    };
    let scheme = if location.protocol().unwrap() == "https:" { "wss" } else { "ws" };
    let url = format!("{scheme}://{host}/api/room/{room_name}/ws");
    let ws = WebSocket::open(&url).unwrap();
    let (write, mut read) = ws.split();
    let (tx, rx) = mpsc::unbounded::<Message>();

    let user = RwSignal::new(load_username());
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

#[component]
fn Home() -> impl IntoView {
    let navigate = use_navigate();
    let (room, set_room) = signal(String::new());
    let (user, set_user) = signal(String::new());

    let join = move |_| {
        save_username(&user.get());
        let name = room.get();
        if !name.trim().is_empty() {
            navigate(&format!("/room/{name}"), Default::default());
        }
    };

    view! {
         <div class="app">
            <h1>"Durable Object Chat"</h1>
            <div class="content">
                <input
                    placeholder="User"
                    prop:value=user
                    on:input=move |ev| set_user.set(event_target_value(&ev))
                />
                <input
                    placeholder="Room"
                    prop:value=room
                    on:input=move |ev| set_room.set(event_target_value(&ev))
                />
                <button on:click=join>"Join"</button>
            </div>
        </div>
    }
}

#[component]
fn App() -> impl IntoView {
    view! {
        <Router>
            <Routes fallback=|| "Page not found.">
                <Route path=path!("/") view=Home />
                <Route path=path!("/room/:name") view=Room />
            </Routes>    
        </Router>
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

const USERNAME_KEY: &str = "chat_username";

fn save_username(name: &str) {
    if let Ok(Some(storage)) = window().local_storage() {
        let _ = storage.set_item(USERNAME_KEY, name);
    }
}

fn load_username() -> String {
    window()
        .local_storage()
        .ok()
        .flatten()
        .and_then(|s| s.get_item(USERNAME_KEY).ok().flatten())
        .unwrap_or_default()
}

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(App);
}
