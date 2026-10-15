use super::super::parse_at_home_pages;
use serde_json::json;

#[test]
fn constructs_original_quality_pages_in_order() {
    let payload = json!({
        "baseUrl": "https://md-home.example",
        "chapter": {
            "hash": "abc123",
            "data": ["001.png", "002.webp"]
        }
    });
    let pages = parse_at_home_pages("chapter-id", &payload).unwrap();
    assert_eq!(pages.len(), 2);
    assert_eq!(pages[0].url, "https://md-home.example/data/abc123/001.png");
    assert_eq!(pages[1].url, "https://md-home.example/data/abc123/002.webp");
    assert_eq!(
        pages[0].referer.as_deref(),
        Some("https://mangadex.org/chapter/chapter-id")
    );
}
