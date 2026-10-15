use super::super::{parse_series_for_site, BASE_URL};
use serde_json::json;

#[test]
fn maps_series_metadata_and_synopsis() {
    let props = json!({
        "manga": {
            "title": "One Piece",
            "slug": "one-piece",
            "synopsis": "A story\n about pirates.",
            "status": "Ongoing",
            "poster": "/storage/one-piece.jpg",
            "type": "Manga",
            "author": "Eiichiro Oda",
            "artist": "-",
            "release_year": 1997,
            "genres": [{"name": "Adventure"}, {"name": "Fantasy"}],
            "chapters": []
        }
    });
    let series = parse_series_for_site(BASE_URL, "one-piece", &props).unwrap();
    assert_eq!(series.title, "One Piece");
    assert_eq!(series.url, "https://westmanga.lol/manga/one-piece");
    assert_eq!(
        series.cover_url,
        "https://westmanga.lol/storage/one-piece.jpg"
    );
    assert_eq!(series.authors, "Eiichiro Oda");
    assert_eq!(series.tags, "Adventure, Fantasy");
    assert_eq!(series.manga_type, "Manga");
    assert_eq!(series.status, "Ongoing");
    assert_eq!(series.released, "1997");
    assert_eq!(series.description, "A story about pirates.");
}
