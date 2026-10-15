use crate::sources::westmanga::parse_search_results_for_site;
use serde_json::json;

#[test]
fn maps_search_results_to_mangaku_domain() {
    let mangas = json!([{
        "slug": "one-piece",
        "title": "One Piece",
        "poster": "https://thumbnail.example/one-piece.jpg",
        "author": "Eiichiro Oda",
        "artist": "-",
        "type": "Manga",
        "status": "Ongoing",
        "release_year": 2026,
        "genres": [{"name": "Adventure"}]
    }]);
    let results =
        parse_search_results_for_site("https://mangaku.lol", mangas.as_array().unwrap(), true);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "One Piece");
    assert_eq!(results[0].url, "https://mangaku.lol/manga/one-piece");
    assert_eq!(
        results[0].cover_url,
        "https://thumbnail.example/one-piece.jpg"
    );
    assert_eq!(results[0].authors, "Eiichiro Oda");
    assert_eq!(results[0].tags, "Adventure");
}
