use super::super::load_series_data;
use serde_json::json;

#[test]
fn maps_series_description_and_metadata() {
    let manga = json!({
        "id": "11111111-1111-4111-8111-111111111111",
        "attributes": {
            "title": {"en": "Example"},
            "description": {"en": "An English synopsis."},
            "status": "completed",
            "year": 2001,
            "originalLanguage": "ko",
            "tags": [{"attributes": {"name": {"en": "Action"}}}]
        },
        "relationships": [
            {"id": "author-id", "type": "author", "attributes": {"name": "Writer"}},
            {"id": "cover-id", "type": "cover_art", "attributes": {"fileName": "cover.jpg"}}
        ]
    });
    let series = load_series_data("11111111-1111-4111-8111-111111111111", &manga, Vec::new());
    assert_eq!(series.title, "Example");
    assert_eq!(series.description, "An English synopsis.");
    assert_eq!(series.authors, "Writer");
    assert_eq!(series.tags, "Action");
    assert_eq!(series.manga_type, "Manhwa");
    assert_eq!(series.status, "completed");
    assert_eq!(series.released, "2001");
}

#[test]
fn null_release_year_is_empty() {
    let manga = json!({
        "attributes": {
            "title": {"en": "No year"},
            "description": {},
            "year": null,
            "originalLanguage": "ja"
        },
        "relationships": []
    });
    let series = load_series_data("11111111-1111-4111-8111-111111111111", &manga, Vec::new());
    assert!(series.released.is_empty());
}
