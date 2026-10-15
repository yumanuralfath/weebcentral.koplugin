use super::super::validate_komiku_slug;

#[test]
fn accepts_percent_encoded_non_ascii_slug() {
    assert!(validate_komiku_slug("37-5%e2%84%83-hidden-love").is_ok());
}

#[test]
fn rejects_encoded_path_separators() {
    assert!(validate_komiku_slug("series%2fchapter").is_err());
    assert!(validate_komiku_slug("series%5cchapter").is_err());
}
