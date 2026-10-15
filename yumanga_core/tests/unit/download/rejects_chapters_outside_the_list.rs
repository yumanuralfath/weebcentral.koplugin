use super::super::parse_selection;

#[test]
fn rejects_chapters_outside_the_list() {
    assert!(parse_selection("0,4", 3).is_err());
    assert!(parse_selection("1-4", 3).is_err());
}
