use super::super::parse_pages;
use super::support::inertia_html;
use crate::models::Chapter;
use serde_json::json;

#[test]
fn missing_chapter_images_is_an_error() {
    let chapter = Chapter {
        id: "chapter-1".into(),
        title: "Chapter 1".into(),
        url: String::new(),
        published_at: String::new(),
    };
    let html = inertia_html(json!({"chapter": {"images": []}}));
    assert!(parse_pages(&chapter, &html).is_err());
}
