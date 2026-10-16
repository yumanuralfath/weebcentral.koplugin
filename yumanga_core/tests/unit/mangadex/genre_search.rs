use super::super::{find_tag_id, prefer_indonesian_translations, search_query};
use serde_json::json;

#[test]
fn genre_only_query_omits_empty_title_and_includes_tag() {
    let query = search_query("", Some("comedy-tag"));

    assert!(!query.iter().any(|(key, _)| key == "title"));
    assert!(
        query
            .iter()
            .any(|(key, value)| { key == "includedTags[]" && value == "comedy-tag" })
    );
}

#[test]
fn finds_genre_tag_without_case_sensitive_input() {
    let payload = json!({
        "data": [{
            "id": "comedy-tag",
            "attributes": { "name": { "en": "Comedy" } }
        }]
    });

    assert_eq!(find_tag_id(&payload, "comedy").unwrap(), "comedy-tag");
}

#[test]
fn prefers_indonesian_per_chapter_and_keeps_english_only_chapters() {
    let chapters = json!([
        {
            "id": "en-chapter-1",
            "attributes": {
                "volume": "1", "chapter": "1", "translatedLanguage": "en",
                "pages": 10, "externalUrl": null, "isUnavailable": false
            }
        },
        {
            "id": "id-chapter-1",
            "attributes": {
                "volume": "1", "chapter": "1", "translatedLanguage": "id",
                "pages": 10, "externalUrl": null, "isUnavailable": false
            }
        },
        {
            "id": "en-chapter-2",
            "attributes": {
                "volume": "1", "chapter": "2", "translatedLanguage": "en",
                "pages": 10, "externalUrl": null, "isUnavailable": false
            }
        }
    ]);

    let selected = prefer_indonesian_translations(chapters.as_array().unwrap().clone());

    assert_eq!(
        selected
            .iter()
            .map(|chapter| chapter["id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["id-chapter-1", "en-chapter-2"]
    );
}
