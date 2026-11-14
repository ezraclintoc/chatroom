use leptos::prelude::*;

const USERNAME_KEY: &str = "chat_username";

pub fn save_username(name: &str) {
    if let Ok(Some(storage)) = window().local_storage() {
        let _ = storage.set_item(USERNAME_KEY, name);
    }
}

pub fn load_username() -> String {
    window()
        .local_storage()
        .ok()
        .flatten()
        .and_then(|s| s.get_item(USERNAME_KEY).ok().flatten())
        .unwrap_or_default()
}
