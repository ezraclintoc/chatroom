use leptos::prelude::*;
use leptos_router::hooks::use_navigate;


use crate::storage::*;

#[component]
pub fn Home() -> impl IntoView {
    let navigate = use_navigate();
    let (user, set_user) = signal(load_username());
    let (room, set_room) = signal(String::new());

    let can_join = move || !user.get().trim().is_empty() && !room.get().trim().is_empty();

    let join = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default(); // stop the browser reloading the page
        if !can_join() {
            return;
        }
        save_username(user.get().trim());
        let room = String::from(js_sys::encode_uri_component(room.get().trim()));
        navigate(&format!("/room/{room}"), Default::default());
    };

    view! {
        <div class="app">
            <form class="join" on:submit=join>
                <h1>"Durable Object Chat"</h1>
                <input
                    placeholder="Username"
                    maxlength="32"
                    prop:value=user
                    on:input=move |ev| set_user.set(event_target_value(&ev))
                />
                <input
                    placeholder="Room"
                    prop:value=room
                    on:input=move |ev| set_room.set(event_target_value(&ev))
                />
                <button type="submit" disabled=move || !can_join()>"Join"</button>
            </form>
        </div>
    }
}
