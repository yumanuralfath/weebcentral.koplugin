use crate::models::Chapter;
use crate::sources::westmanga::parse_pages_for_site;

#[test]
fn resolves_mangaku_reader_images_in_order() {
    let chapter = Chapter {
        id: "chapter-1194".into(),
        title: "Chapter 1194".into(),
        url: "https://mangaku.lol/manga/one-piece/chapter/1194".into(),
        published_at: String::new(),
    };
    let html = r#"<div id="app" data-page="{&quot;props&quot;:{&quot;chapter&quot;:{&quot;images&quot;:[{&quot;order&quot;:2,&quot;image_path&quot;:&quot;https://img.example/2.webp&quot;},{&quot;order&quot;:0,&quot;image_path&quot;:&quot;https://image2.komiku.to/komiku-promosi.webp&quot;},{&quot;order&quot;:1,&quot;image_path&quot;:&quot;https://img.example/1.webp&quot;}]}}}"></div>"#;
    let pages = parse_pages_for_site("https://mangaku.lol", &chapter, html).unwrap();
    assert_eq!(pages.len(), 2);
    assert_eq!(pages[0].url, "https://img.example/1.webp");
    assert_eq!(pages[1].url, "https://img.example/2.webp");
    assert_eq!(pages[0].referer.as_deref(), Some(chapter.url.as_str()));
}
