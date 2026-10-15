use super::super::parse_search_results;
use serde_json::json;

#[test]
fn maps_search_result_fields_and_optional_details() {
    let manga = json!({
        "slug": "one-piece",
        "title": "One Piece",
        "poster": "https://cdn.example/one-piece.jpg",
        "author": "Eiichiro Oda",
        "artist": "Eiichiro Oda",
        "type": "Manga",
        "status": "Ongoing",
        "release_year": 1997,
        "genres": [{"name": "Action"}, {"name": "Adventure"}]
    });
    let results = parse_search_results(&[manga.clone()], true);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "one-piece");
    assert_eq!(results[0].title, "One Piece");
    assert_eq!(results[0].url, "https://westmanga.lol/manga/one-piece");
    assert_eq!(results[0].cover_url, "https://cdn.example/one-piece.jpg");
    assert_eq!(results[0].authors, "Eiichiro Oda");
    assert_eq!(results[0].tags, "Action, Adventure");
    assert_eq!(results[0].manga_type, "Manga");
    assert_eq!(results[0].status, "Ongoing");
    assert_eq!(results[0].released, "1997");

    let results = parse_search_results(&[manga], false);
    assert!(results[0].authors.is_empty());
    assert!(results[0].tags.is_empty());
}
