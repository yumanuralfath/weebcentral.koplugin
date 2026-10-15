use super::super::{parse_pages, Chapter};

#[test]
fn derives_every_page_url_from_the_first_one() {
    let chapter = Chapter {
        id: "ABC".into(),
        title: "Chapter 1".into(),
        url: String::new(),
        published_at: String::new(),
    };
    let html = r#"<html><head>
        <link rel="preload" href="https://cdn.example/m/001-009.png" as="image">
        </head><body>
        x-show="page === 1 ? a" x-show="page === 3 ? b"
        </body></html>"#;
    let pages = parse_pages(&chapter, html).unwrap();
    let urls: Vec<_> = pages.iter().map(|page| page.url.as_str()).collect();
    assert_eq!(
        urls,
        [
            "https://cdn.example/m/001-001.png",
            "https://cdn.example/m/001-002.png",
            "https://cdn.example/m/001-003.png"
        ]
    );
    assert_eq!(
        pages[0].referer.as_deref(),
        Some("https://weebcentral.com/chapters/ABC")
    );
}
