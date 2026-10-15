use super::super::{parse_pages, Chapter};

#[test]
fn reads_page_images_in_order_and_ignores_the_promo_image() {
    let chapter = Chapter {
        id: "one-piece-chapter-1195".into(),
        title: "Chapter 1195".into(),
        url: "https://komiku.org/one-piece-chapter-1195/".into(),
        published_at: String::new(),
    };
    let html = r#"<img src="https://image2.komiku.to/komiku-promosi.webp"><img class="klazy ww" id="2" src="https://image3.komiku.to/upload5/one-piece/1195/2.webp"><img class="klazy ww" id="1" src="https://image2.komiku.to/upload5/one-piece/1195/1.webp">"#;
    let pages = parse_pages(&chapter, html).unwrap();
    assert_eq!(pages.len(), 2);
    assert_eq!(
        pages[0].url,
        "https://image3.komiku.to/upload5/one-piece/1195/2.webp"
    );
    assert_eq!(
        pages[1].url,
        "https://image2.komiku.to/upload5/one-piece/1195/1.webp"
    );
    assert!(pages
        .iter()
        .all(|page| page.referer.as_deref() == Some(chapter.url.as_str())));
}
