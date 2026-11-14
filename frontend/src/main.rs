mod pages;
mod storage;
mod time;

use leptos::prelude::*;
use leptos_router::components::{Route, Router, Routes};
use leptos_router::path;
use pages::{home::*, room::*};

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

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(App);
}
