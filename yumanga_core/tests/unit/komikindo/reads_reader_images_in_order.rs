use super::super::{parse_pages, Chapter};

#[test]
fn reads_reader_images_in_order_without_collecting_ads() {
    let chapter = Chapter {
        id: "eleceed-chapter-1".into(),
        title: "Chapter 1".into(),
        url: "https://komikindo.ch/eleceed-chapter-1/".into(),
        published_at: String::new(),
    };
    let html = r#"<img src="https://example.com/ad.jpg"><div id="Baca_Komik"><img src="//cdn.example/001.webp"><img src="//cdn.example/002.webp"><img src="//cdn.example/002.webp"></div>"#;
    let pages = parse_pages(&chapter, html).unwrap();
    assert_eq!(pages.len(), 2);
    assert_eq!(pages[0].url, "https://cdn.example/001.webp");
    assert_eq!(pages[1].url, "https://cdn.example/002.webp");
    assert_eq!(pages[0].referer.as_deref(), Some(chapter.url.as_str()));
}
