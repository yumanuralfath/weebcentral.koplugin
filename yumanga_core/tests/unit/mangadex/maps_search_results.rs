use super::super::parse_search_results;
use serde_json::json;

#[test]
fn maps_title_cover_and_optional_metadata() {
    let manga = json!({
        "id": "11111111-1111-4111-8111-111111111111",
        "attributes": {
            "title": {"ja": "One Piece", "en": "One Piece"},
            "status": "ongoing",
            "year": 1997,
            "originalLanguage": "ja",
            "tags": [{"attributes": {"name": {"en": "Action"}}}]
        },
        "relationships": [
            {"id": "cover-id", "type": "cover_art", "attributes": {"fileName": "cover.png"}},
            {"id": "author-id", "type": "author", "attributes": {"name": "Eiichiro Oda"}},
            {"id": "artist-id", "type": "artist", "attributes": {"name": "Eiichiro Oda"}}
        ]
    });
    let results = parse_search_results(&[manga.clone()], true);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "11111111-1111-4111-8111-111111111111");
    assert_eq!(results[0].title, "One Piece");
    assert_eq!(
        results[0].url,
        "https://mangadex.org/title/11111111-1111-4111-8111-111111111111"
    );
    assert_eq!(
        results[0].cover_url,
        "https://uploads.mangadex.org/covers/11111111-1111-4111-8111-111111111111/cover.png.256.jpg"
    );
    assert_eq!(results[0].authors, "Eiichiro Oda");
    assert_eq!(results[0].tags, "Action");
    assert_eq!(results[0].manga_type, "Manga");
    assert_eq!(results[0].status, "ongoing");
    assert_eq!(results[0].released, "1997");

    let results = parse_search_results(&[manga], false);
    assert!(results[0].authors.is_empty());
    assert!(results[0].tags.is_empty());
}
