use super::super::download_selection;
use super::support::{entry_names, temp_dir, MockSource};

#[test]
fn merges_chapters_into_one_volume() {
    let dir = temp_dir("volume");
    let progress = dir.join("p.progress");
    let result = download_selection(
        &MockSource,
        "series",
        "1-3",
        "Vol 1",
        dir.to_str().unwrap(),
        progress.to_str().unwrap(),
    )
    .unwrap();
    assert_eq!(result.paths.len(), 1);

    let names = entry_names(&result.paths[0]);
    assert_eq!(names.len(), 6);
    assert_eq!(names[0], "0001 - Chapter 1/0001.png");
    assert_eq!(names[5], "0003 - Chapter 3/0002.png");
    let _ = std::fs::remove_dir_all(&dir);
}
