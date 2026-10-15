use super::super::{parse_metadata, Html};

#[test]
fn parses_authors_tags_status_and_type() {
    let html = r#"<div class="infox"><div class="spe"><span><b>Status:</b> Berjalan</span><span><b>Pengarang:</b> Son Jae-Ho</span><span><b>Ilustrator:</b> Zhena</span><span><b>Tema:</b> Animals</span><span><b>Jenis Komik:</b> Manhwa</span></div></div>"#;
    let metadata = parse_metadata(&Html::parse_document(html));
    assert_eq!(metadata.authors, "Son Jae-Ho, Zhena");
    assert_eq!(metadata.tags, "Animals");
    assert_eq!(metadata.status, "Berjalan");
    assert_eq!(metadata.manga_type, "Manhwa");
}
