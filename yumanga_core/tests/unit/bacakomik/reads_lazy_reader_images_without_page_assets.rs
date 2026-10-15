use super::super::parse_pages;
use crate::models::Chapter;

#[test]
fn reads_lazy_reader_images_and_skips_svg_placeholders_and_cover() {
    let chapter = Chapter {
        id: "one-piece-chapter-1195".into(),
        title: "Chapter 1195".into(),
        url: "https://bacakomik.my/one-piece-chapter-1195/".into(),
        published_at: String::new(),
    };
    let html = r#"<img src="https://bacakomik.my/wp-content/uploads/cover.png"><div class="reader"><img src="data:image/svg+xml,%3Csvg%3E" data-lazy-src="https://reader.example/media/3142159/chapter-1195/001.webp"><img src="data:image/svg+xml,%3Csvg%3E" data-lazy-src="https://reader.example/media/3142159/chapter-1195/002.webp"></div>"#;
    let pages = parse_pages(&chapter, html).unwrap();
    assert_eq!(pages.len(), 2);
    assert_eq!(
        pages[0].url,
        "https://reader.example/media/3142159/chapter-1195/001.webp"
    );
    assert_eq!(
        pages[1].url,
        "https://reader.example/media/3142159/chapter-1195/002.webp"
    );
    assert!(pages
        .iter()
        .all(|page| page.referer.as_deref() == Some(chapter.url.as_str())));
}
