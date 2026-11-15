use leptos::either::Either;
use leptos::prelude::*;
use leptos_router::components::{Redirect, A};
use leptos_router::hooks::use_params_map;
use std::collections::BTreeMap;
use std::time::Duration;

use futures::StreamExt;
use futures::channel::mpsc;
use gloo_net::websocket::{Message, futures::WebSocket};
use leptos::task::spawn_local;
use shared::*;

use crate::storage::*;
use crate::time::*;

#[component]
pub fn Room() -> impl IntoView {
    if load_username().trim().is_empty() {
        Either::Left(view! { <Redirect path="/" /> })
    } else {
        Either::Right(view! { <ChatRoom /> })
    }
}

#[component]
pub fn ChatRoom() -> impl IntoView {
    let params = use_params_map();
    let room_name = params.read_untracked().get("name").unwrap_or_default();

    let location = window().location();
    let host = if location.port().unwrap() == "8080" {
        "localhost:8787".to_string() // dev: Trunk on 8080, Wrangler on 8787
    } else {
        location.host().unwrap() //prod
    };
    let scheme = if location.protocol().unwrap() == "https:" {
        "wss"
    } else {
        "ws"
    };
    let url = format!("{scheme}://{host}/api/room/{room_name}/ws");
    let ws = WebSocket::open(&url).unwrap();
    let (write, mut read) = ws.split();
    let (tx, rx) = mpsc::unbounded::<Message>();

    let user = RwSignal::new(load_username());
    let (draft, set_draft) = signal(String::new());

    let messages = RwSignal::new(Vec::<ChatMessage>::new());
    let typing = RwSignal::new(BTreeMap::<String, i64>::new());
    let error = RwSignal::new(None::<String>);
    let status = RwSignal::new("connecting");
    
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
                Ok(ServerMsg::Message(m)) => {
                    typing.update(|u| u.retain(|name, _t| *name != m.username));
                    messages.update(|v| v.push(m))
                }
                Ok(ServerMsg::Error { reason }) => error.set(Some(reason)),
                Ok(ServerMsg::Typing { username }) => typing.update(|u| {
                    u.insert(username, js_sys::Date::now() as i64);
                }),
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
    let messages_ref = NodeRef::<leptos::html::Div>::new();
    let stick_to_bottom = StoredValue::new(true);
    
    Effect::new(move |_| {
        messages.track(); // rerun this whenever messages changes
        if stick_to_bottom.get_value() {
            if let Some(el) = messages_ref.get() {
                el.set_scroll_top(el.scroll_height());
            }
        }
    });
    
    view! {
        <div class="app">
            <div class="chat">
                <div class="chat-header">
                    <div class="room-info">
                        <span class="room-name">"#" {room_name.clone()}</span>
                        <span class=move || format!("status {}", status.get())>
                            {move || match status.get() {
                                "connected" => "Connected",
                                "disconnected" => "Disconnected",
                                _ => "Connecting...",
                            }}
                        </span>
                    </div>
                    <A href="/">"Leave"</A>
                </div>
                <div
                    class="messages"
                    node_ref=messages_ref
                    on:scroll=move |_| {
                        if let Some(el) = messages_ref.get() {
                            let distance = el.scroll_height() - el.scroll_top() - el.client_height();
                            stick_to_bottom.set_value(distance < 50);
                        }
                    }
                >
                    <Show when=move || messages.with(|v| v.is_empty())>
                        <div class="empty">
                            <strong>"No messages yet"</strong>
                            "Be the first to say hi"
                        </div>
                    </Show>
                    <For
                        each=move || {
                            messages.with(|v| {
                                v.iter()
                                    .enumerate()
                                    .map(|(i, m)| {
                                        let show_name = i == 0 || v[i - 1].username != m.username;
                                        (m.clone(), show_name)
                                    })
                                    .collect::<Vec<_>>()
                            })
                        }
                        key=|(m, show_name)| (m.id, *show_name)
                        children=move |(m, show_name)| {
                            let color = avatar_color(&m.username);
                            let is_mine = m.username == user.get_untracked();
                            view! {
                                <div class="message" class:grouped=!show_name class:mine=is_mine>
                                    <div class="avatar-slot">
                                        {show_name.then(|| view! {
                                            <div class="avatar" style:background-color=color.clone()>
                                                {initial(&m.username)}
                                            </div>
                                        })}
                                    </div>
                                    <div class="message-main">
                                        {show_name.then(|| view! {
                                            <div class="username" style:color=color.clone()>{m.username.clone()}</div>
                                        })}
                                        <div class="body">{m.body.clone()}</div>
                                    </div>
                                    <span class="time">{format_time(m.sent_at)}</span>
                                </div>
                            }
                        }
                    />
                </div>
                <div class="typing">
                    {move || {
                         let names: Vec<String> = typing.with(|v| v.iter().map(|(n, _)| n.clone()).collect());
                        let text = match names.len() {
                            0 => String::new(),
                            1 => format!("{} is typing", names[0]),
                            2 => format!("{} and {} are typing", names[0], names[1]),
                            _ => "Several people are typing".to_string(),
                        };
                        (!text.is_empty()).then(|| view! {
                            <span>{text}</span>
                            <span class="dots"><i></i><i></i><i></i></span>
                        })
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
