use super::super::{parse_pages, Chapter};

#[test]
fn missing_preload_link_is_an_error() {
    let chapter = Chapter {
        id: "ABC".into(),
        title: String::new(),
        url: String::new(),
        published_at: String::new(),
    };
    assert!(parse_pages(&chapter, "<html></html>").is_err());
}
