use wasm_bindgen::JsValue;

pub fn format_time(ms: i64) -> String {
    let date = js_sys::Date::new(&JsValue::from_f64(ms as f64));
    let opts = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&opts, &"hour".into(), &"numeric".into());
    let _ = js_sys::Reflect::set(&opts, &"minute".into(), &"2-digit".into());
    date.to_locale_time_string_with_options("en-US", &opts)
        .into()
}
