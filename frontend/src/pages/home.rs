use leptos::prelude::*;
use leptos_router::hooks::use_navigate;


use crate::storage::*;

#[component]
pub fn Home() -> impl IntoView {
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
