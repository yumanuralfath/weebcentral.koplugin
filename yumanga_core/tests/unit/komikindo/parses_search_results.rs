use super::super::parse_search_results;
use super::super::Html;

#[test]
fn parses_search_result_id_title_and_cover() {
    let html = r#"<div class="listupd"><div class="animepost"><div class="animposx"><a href="https://komikindo.ch/komik/846048-eleceed/" itemprop="url" title="Komik Eleceed"><img src="/cover.jpg"></a></div></div></div>"#;
    let results = parse_search_results(&Html::parse_document(html));
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "846048-eleceed");
    assert_eq!(results[0].title, "Eleceed");
    assert_eq!(results[0].cover_url, "https://komikindo.ch/cover.jpg");
}
