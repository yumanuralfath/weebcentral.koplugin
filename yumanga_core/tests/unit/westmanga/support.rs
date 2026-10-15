use serde_json::Value;

pub(super) fn inertia_html(props: Value) -> String {
    let payload = serde_json::json!({
        "component": "Manga/Read",
        "props": props,
        "url": "/manga/example/chapter/1",
        "version": "test"
    });
    let encoded = serde_json::to_string(&payload)
        .unwrap()
        .replace('&', "&amp;")
        .replace('"', "&quot;");
    format!("<div id=\"app\" data-page=\"{encoded}\"></div>")
}
