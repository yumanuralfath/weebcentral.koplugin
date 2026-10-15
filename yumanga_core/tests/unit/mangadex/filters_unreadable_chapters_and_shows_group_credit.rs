use super::super::parse_chapters;
use serde_json::json;

#[test]
fn filters_external_unavailable_and_empty_chapters_and_credits_group() {
    let chapters = json!([
        {
            "id": "chapter-1",
            "attributes": {"volume": "1", "chapter": "1", "title": "Start", "pages": 12, "publishAt": "2024-01-01T00:00:00+00:00", "externalUrl": null, "isUnavailable": false},
            "relationships": [{"type": "scanlation_group", "attributes": {"name": "Group A"}}]
        },
        {
            "id": "chapter-external",
            "attributes": {"chapter": "2", "pages": 0, "externalUrl": "https://outside.example", "isUnavailable": false},
            "relationships": []
        },
        {
            "id": "chapter-unavailable",
            "attributes": {"chapter": "3", "pages": 20, "externalUrl": null, "isUnavailable": true},
            "relationships": []
        },
        {
            "id": "chapter-empty",
            "attributes": {"chapter": "4", "pages": 0, "externalUrl": null, "isUnavailable": false},
            "relationships": []
        }
    ]);
    let parsed = parse_chapters(chapters.as_array().unwrap());
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].id, "chapter-1");
    assert_eq!(parsed[0].title, "Vol. 1 Ch. 1 - Start [Group A]");
    assert_eq!(parsed[0].url, "https://mangadex.org/chapter/chapter-1");
    assert_eq!(parsed[0].published_at, "2024-01-01T00:00:00+00:00");
}
