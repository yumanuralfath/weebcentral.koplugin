use super::super::parse_selection;

#[test]
fn parses_single_and_discrete_chapters() {
    assert_eq!(parse_selection("1,3,4", 10).unwrap(), vec![0, 2, 3]);
}
