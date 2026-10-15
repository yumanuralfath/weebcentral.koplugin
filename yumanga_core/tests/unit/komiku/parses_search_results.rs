use super::super::{parse_search_results, Html};

#[test]
fn parses_search_result_title_slug_and_lazy_cover() {
    let html = r#"<div class="bge"><div class="bgei"><a href="/manga/komik-one-piece-indo/"><img src="/asset/img/lazy.jpg" data-src="https://thumbnail.komiku.to/one-piece.jpg"></a></div><div class="kan"><a href="/manga/komik-one-piece-indo/"><h3>One Piece</h3></a></div></div>"#;
    let results = parse_search_results(&Html::parse_document(html));
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "komik-one-piece-indo");
    assert_eq!(results[0].title, "One Piece");
    assert_eq!(
        results[0].url,
        "https://komiku.org/manga/komik-one-piece-indo/"
    );
    assert_eq!(
        results[0].cover_url,
        "https://thumbnail.komiku.to/one-piece.jpg"
    );
}
