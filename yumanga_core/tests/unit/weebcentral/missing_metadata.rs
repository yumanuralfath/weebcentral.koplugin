use super::super::{parse_series_meta, Html};

#[test]
fn missing_metadata_is_empty() {
    let metadata = parse_series_meta(&Html::parse_document("<p>nothing</p>"));
    assert!(metadata.authors.is_empty() && metadata.released.is_empty());
}
