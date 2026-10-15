use super::super::{Html, parse_series_meta};

#[test]
fn parses_series_metadata_labels() {
    let page = r#"<ul>
        <li><strong>Author(s): </strong><span><a>Eiichiro Oda</a></span></li>
        <li><strong>Tags(s): </strong><span><a>Action</a>, <a>Adventure</a></span></li>
        <li><strong>Type: </strong><a>Manga</a></li>
        <li><strong>Status: </strong><a>Ongoing</a></li>
        <li><strong>Released: </strong><span>1997</span></li>
    </ul>"#;
    let metadata = parse_series_meta(&Html::parse_document(page));
    assert_eq!(metadata.authors, "Eiichiro Oda");
    assert_eq!(metadata.tags, "Action, Adventure");
    assert_eq!(metadata.manga_type, "Manga");
    assert_eq!(metadata.status, "Ongoing");
    assert_eq!(metadata.released, "1997");
}
