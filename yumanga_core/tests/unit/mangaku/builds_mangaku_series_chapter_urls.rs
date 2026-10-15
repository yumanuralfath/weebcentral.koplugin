use crate::sources::westmanga::parse_series_for_site;
use serde_json::json;

#[test]
fn builds_series_and_chapter_urls_for_mangaku() {
    let props = json!({
        "manga": {
            "title": "Example Manga",
            "synopsis": "A short synopsis.",
            "status": "Ongoing",
            "poster": "https://thumbnail.example/cover.jpg",
            "type": "Manga",
            "author": "Author Name",
            "artist": "-",
            "release_year": 2020,
            "genres": [{"name": "Fantasy"}],
            "chapters": [
                {"title": "Chapter 2", "slug": "chapter-2", "chapter_number": 2, "created_at": "2026-02-01"},
                {"title": "Chapter 1", "slug": "chapter-1", "chapter_number": 1, "created_at": "2026-01-01"}
            ]
        }
    });
    let series = parse_series_for_site("https://mangaku.lol", "example-manga", &props).unwrap();
    assert_eq!(series.url, "https://mangaku.lol/manga/example-manga");
    assert_eq!(series.chapters.len(), 2);
    assert_eq!(series.chapters[0].id, "chapter-1");
    assert_eq!(
        series.chapters[0].url,
        "https://mangaku.lol/manga/example-manga/chapter/1"
    );
    assert_eq!(series.chapters[1].id, "chapter-2");
}
