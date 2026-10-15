use super::super::chapter_filename;

#[test]
fn generates_manga_filename_without_zero_padding() {
    assert_eq!(chapter_filename(0, "One Piece"), "1 - One Piece.cbz");
    assert_eq!(chapter_filename(1, "One Piece"), "2 - One Piece.cbz");
    assert_eq!(chapter_filename(9, "One Piece"), "10 - One Piece.cbz");
}
