use super::super::{parse_pages, Chapter};

#[test]
fn missing_reader_images_is_an_error() {
    let chapter = Chapter {
        id: "eleceed-chapter-1".into(),
        title: "Chapter 1".into(),
        url: String::new(),
        published_at: String::new(),
    };
    assert!(parse_pages(&chapter, "<html></html>").is_err());
}
