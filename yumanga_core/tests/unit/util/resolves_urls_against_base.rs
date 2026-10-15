use super::super::absolute_url;

#[test]
fn resolves_urls_against_the_base() {
    let base = "https://example.com";
    assert_eq!(absolute_url(base, "/a/b"), "https://example.com/a/b");
    assert_eq!(absolute_url(base, "//cdn.x/y.png"), "https://cdn.x/y.png");
    assert_eq!(absolute_url(base, "https://z/y"), "https://z/y");
    assert_eq!(absolute_url(base, ""), "");
}
