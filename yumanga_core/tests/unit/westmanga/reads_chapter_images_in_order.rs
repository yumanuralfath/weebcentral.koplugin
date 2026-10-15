use super::super::{parse_pages, Chapter};
use super::support::inertia_html;
use serde_json::json;

#[test]
fn sorts_images_by_order_and_skips_thumbnails() {
    let chapter = Chapter {
        id: "chapter-1".into(),
        title: "Chapter 1".into(),
        url: "https://westmanga.lol/manga/one-piece/chapter/1".into(),
        published_at: String::new(),
    };
    let html = inertia_html(json!({
        "chapter": {
            "images": [
                {"order": 2, "image_path": "https://img.example/002.webp"},
                {"order": 0, "image_path": "https://img.example/promo.webp"},
                {"order": 1, "image_path": "https://img.example/001.webp"},
                {"order": 3, "image_path": "https://img.example/banner-300x300.jpg"}
            ]
        }
    }));
    let pages = parse_pages(&chapter, &html).unwrap();
    assert_eq!(pages.len(), 3);
    assert_eq!(pages[0].url, "https://img.example/promo.webp");
    assert_eq!(pages[1].url, "https://img.example/001.webp");
    assert_eq!(pages[2].url, "https://img.example/002.webp");
    assert_eq!(pages[0].referer.as_deref(), Some(chapter.url.as_str()));
}
