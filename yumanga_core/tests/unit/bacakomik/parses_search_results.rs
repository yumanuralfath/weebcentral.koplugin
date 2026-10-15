use super::super::{parse_search_results, Html};

#[test]
fn parses_bacakomik_series_card() {
    let html = r#"<div class="film-list"><div class="animepost"><div class="animposx"><a href="https://bacakomik.my/komik/one-piece/" itemprop="url" title="Komik One Piece"><img src="https://i2.wp.com/bacakomik.my/cover.png" title="Komik One Piece" alt="Komik One Piece" itemprop="image"></a></div></div></div>"#;
    let results = parse_search_results(&Html::parse_document(html));
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "one-piece");
    assert_eq!(results[0].title, "One Piece");
    assert_eq!(results[0].url, "https://bacakomik.my/komik/one-piece/");
    assert_eq!(
        results[0].cover_url,
        "https://i2.wp.com/bacakomik.my/cover.png"
    );
}
