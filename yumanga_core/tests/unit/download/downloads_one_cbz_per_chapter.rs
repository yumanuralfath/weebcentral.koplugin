use super::super::download_selection;
use super::support::{entry_names, temp_dir, MockSource};

#[test]
fn downloads_one_cbz_per_chapter_from_any_source() {
    let dir = temp_dir("single");
    let progress = dir.join("p.progress");
    let result = download_selection(
        &MockSource,
        "series",
        "2-3",
        "",
        dir.to_str().unwrap(),
        progress.to_str().unwrap(),
    )
    .unwrap();
    assert_eq!(result.paths.len(), 2);
    assert!(result.paths[0].ends_with("2 - Mock Manga.cbz"));
    assert_eq!(
        entry_names(&result.paths[0]),
        ["Chapter 2/0001.png", "Chapter 2/0002.png"]
    );
    let _ = std::fs::remove_dir_all(&dir);
}
