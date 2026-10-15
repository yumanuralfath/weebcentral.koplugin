use super::super::image_extension;

#[test]
fn extracts_image_extensions() {
    assert_eq!(image_extension("https://x/a/001.PNG?v=2"), ".png");
    assert_eq!(image_extension("https://x/a/001.webp"), ".webp");
    assert_eq!(image_extension("https://x/a/001"), ".jpg");
    assert_eq!(image_extension("https://x.com/img"), ".jpg");
}
