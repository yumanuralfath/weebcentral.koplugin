use super::super::parse_series;
use serde_json::json;

#[test]
fn returns_chapters_in_ascending_number_order_with_internal_reader_urls() {
    let props = json!({
        "manga": {
            "title": "Example",
            "chapters": [
                {"title": "Chapter 3", "slug": "chapter-3", "chapter_number": 3, "created_at": "2026-03-03"},
                {"title": "Chapter 2", "slug": "chapter-2", "chapter_number": 2, "created_at": "2026-03-02"},
                {"title": "Chapter 1", "slug": "chapter-1", "chapter_number": 1, "created_at": "2026-03-01"}
            ]
        }
    });
    let series = parse_series("one-piece", &props).unwrap();
    assert_eq!(series.chapters.len(), 3);
    assert_eq!(series.chapters[0].id, "chapter-1");
    assert_eq!(
        series.chapters[0].url,
        "https://westmanga.lol/manga/one-piece/chapter/1"
    );
    assert_eq!(series.chapters[2].id, "chapter-3");
    assert_eq!(series.chapters[2].published_at, "2026-03-03");
}
