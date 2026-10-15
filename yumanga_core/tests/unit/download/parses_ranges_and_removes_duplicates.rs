use super::super::parse_selection;

#[test]
fn parses_ranges_and_removes_duplicates() {
    assert_eq!(parse_selection("1-3,3,5", 10).unwrap(), vec![0, 1, 2, 4]);
}
