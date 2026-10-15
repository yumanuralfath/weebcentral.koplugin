use super::super::parse_at_home_pages;
use serde_json::json;

#[test]
fn missing_at_home_page_list_is_an_error() {
    let payload =
        json!({"baseUrl": "https://md-home.example", "chapter": {"hash": "abc123", "data": []}});
    assert!(parse_at_home_pages("chapter-id", &payload).is_err());
}
