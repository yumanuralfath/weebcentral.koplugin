use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::models::{Chapter, DownloadResult, PageImage, SeriesChapters};
use crate::sources::Source;
use crate::util::{image_extension, safe_filename};

/// A chapter together with its resolved page images.
type Prepared<'a> = (&'a Chapter, Vec<PageImage>);

pub fn download_selection(
    source: &dyn Source,
    series_id: &str,
    selection: &str,
    volume_name: &str,
    output_dir: &str,
    progress_path: &str,
) -> Result<DownloadResult, String> {
    let progress_path = Path::new(progress_path);
    if let Some(progress_directory) = progress_path.parent() {
        fs::create_dir_all(progress_directory).map_err(|error| error.to_string())?;
    }
    write_progress(progress_path, "scan", 0, 1, "Loading chapter list");
    let series = source.series(series_id)?;
    let selected_indices = parse_selection(selection, series.chapters.len())?;
    let output_dir = Path::new(output_dir);
    fs::create_dir_all(output_dir).map_err(|error| error.to_string())?;

    let result = if volume_name.trim().is_empty() {
        download_individual_chapters(
            source,
            &series,
            &series.chapters,
            &selected_indices,
            output_dir,
            progress_path,
        )
    } else {
        download_merged_volume(
            source,
            &series,
            &series.chapters,
            &selected_indices,
            volume_name,
            output_dir,
            progress_path,
        )
    };
    let _ = fs::remove_file(progress_path);
    result
}

pub fn parse_selection(selection: &str, chapter_count: usize) -> Result<Vec<usize>, String> {
    let mut selected = Vec::new();
    let mut seen = vec![false; chapter_count];

    for part in selection.split(',').map(str::trim) {
        if part.is_empty() {
            return Err("Chapter selection contains an empty entry".to_string());
        }
        let range = if let Some((start, end)) = part.split_once('-') {
            let start = parse_chapter_number(start, chapter_count)?;
            let end = parse_chapter_number(end, chapter_count)?;
            if start > end {
                return Err(format!("Invalid chapter range: {part}"));
            }
            start..=end
        } else {
            let chapter = parse_chapter_number(part, chapter_count)?;
            chapter..=chapter
        };

        for chapter_number in range {
            let index = chapter_number - 1;
            if !seen[index] {
                selected.push(index);
                seen[index] = true;
            }
        }
    }

    if selected.is_empty() {
        return Err("Select at least one chapter".to_string());
    }
    Ok(selected)
}

fn parse_chapter_number(value: &str, chapter_count: usize) -> Result<usize, String> {
    let number = value
        .trim()
        .parse::<usize>()
        .map_err(|_| format!("Invalid chapter number: {value}"))?;
    if number == 0 || number > chapter_count {
        return Err(format!("Chapter {number} is outside 1-{chapter_count}"));
    }
    Ok(number)
}

fn download_individual_chapters(
    source: &dyn Source,
    series_chapter: &SeriesChapters,
    chapters: &[Chapter],
    selected_indices: &[usize],
    output_dir: &Path,
    progress_path: &Path,
) -> Result<DownloadResult, String> {
    let mut paths = Vec::new();
    let mut skipped = 0;
    let mut jobs = Vec::new();

    for chapter_index in selected_indices {
        let chapter = &chapters[*chapter_index];

        let path = output_dir.join(chapter_filename(
            *chapter_index,
            &safe_filename(&series_chapter.title),
        ));

        if path.is_file() {
            skipped += 1;
            paths.push(path.to_string_lossy().into_owned());
            continue;
        }
        jobs.push((chapter, path));
    }

    let prepared = prepare_chapters(
        source,
        jobs.iter().map(|(chapter, _)| *chapter),
        progress_path,
    )?;
    let total_images = prepared
        .iter()
        .map(|(_, pages)| pages.len())
        .sum::<usize>()
        .max(1);
    let mut completed_images = 0;
    write_progress(
        progress_path,
        "download",
        0,
        total_images,
        "Preparing download",
    );

    for ((chapter, path), (_, pages)) in jobs.iter().zip(prepared.iter()) {
        let temporary_path = path.with_extension("cbz.part");
        if let Err(error) = write_chapter_cbz(
            source,
            chapter,
            pages,
            &temporary_path,
            progress_path,
            &mut completed_images,
            total_images,
        ) {
            let _ = fs::remove_file(&temporary_path);
            return Err(error);
        }
        fs::rename(&temporary_path, path).map_err(|error| error.to_string())?;
        paths.push(path.to_string_lossy().into_owned());
    }
    write_progress(
        progress_path,
        "download",
        total_images,
        total_images,
        "Complete",
    );

    Ok(DownloadResult { paths, skipped })
}

fn download_merged_volume(
    source: &dyn Source,
    series_chapter: &SeriesChapters,
    chapters: &[Chapter],
    selected_indices: &[usize],
    volume_name: &str,
    output_dir: &Path,
    progress_path: &Path,
) -> Result<DownloadResult, String> {
    let path = output_dir.join(format!(
        "{} - {}.cbz",
        series_chapter.title,
        safe_filename(volume_name)
    ));
    if path.is_file() {
        return Ok(DownloadResult {
            paths: vec![path.to_string_lossy().into_owned()],
            skipped: 1,
        });
    }

    let temporary_path = path.with_extension("cbz.part");
    let selected_chapters = selected_indices
        .iter()
        .map(|index| &chapters[*index])
        .collect::<Vec<_>>();
    let prepared = prepare_chapters(source, selected_chapters.iter().copied(), progress_path)?;
    let total_images = prepared
        .iter()
        .map(|(_, pages)| pages.len())
        .sum::<usize>()
        .max(1);
    write_progress(
        progress_path,
        "download",
        0,
        total_images,
        "Preparing download",
    );
    let result = write_volume_cbz(
        source,
        &prepared,
        &temporary_path,
        progress_path,
        total_images,
    );
    if let Err(error) = result {
        let _ = fs::remove_file(&temporary_path);
        return Err(error);
    }
    fs::rename(&temporary_path, &path).map_err(|error| error.to_string())?;
    Ok(DownloadResult {
        paths: vec![path.to_string_lossy().into_owned()],
        skipped: 0,
    })
}

fn write_volume_cbz(
    source: &dyn Source,
    prepared: &[Prepared],
    path: &Path,
    progress_path: &Path,
    total_images: usize,
) -> Result<(), String> {
    let file = File::create(path).map_err(|error| error.to_string())?;
    let mut archive = ZipWriter::new(file);
    let options = FileOptions::default().compression_method(CompressionMethod::Deflated);

    let mut completed_images = 0;
    for (index, (chapter, pages)) in prepared.iter().enumerate() {
        // Prefix keeps reading order correct and folder names unique.
        let folder = format!("{:04} - {}", index + 1, safe_filename(&chapter.title));
        append_chapter_images(
            source,
            chapter,
            pages,
            &folder,
            &mut archive,
            options,
            progress_path,
            &mut completed_images,
            total_images,
        )?;
    }
    archive.finish().map_err(|error| error.to_string())?;
    write_progress(
        progress_path,
        "download",
        total_images,
        total_images,
        "Complete",
    );
    Ok(())
}

fn write_chapter_cbz(
    source: &dyn Source,
    chapter: &Chapter,
    pages: &[PageImage],
    path: &Path,
    progress_path: &Path,
    completed_images: &mut usize,
    total_images: usize,
) -> Result<(), String> {
    let file = File::create(path).map_err(|error| error.to_string())?;
    let mut archive = ZipWriter::new(file);
    let options = FileOptions::default().compression_method(CompressionMethod::Deflated);
    let folder = safe_filename(&chapter.title);
    append_chapter_images(
        source,
        chapter,
        pages,
        &folder,
        &mut archive,
        options,
        progress_path,
        completed_images,
        total_images,
    )?;
    archive.finish().map_err(|error| error.to_string())?;
    Ok(())
}

fn prepare_chapters<'a>(
    source: &dyn Source,
    chapters: impl Iterator<Item = &'a Chapter>,
    progress_path: &Path,
) -> Result<Vec<Prepared<'a>>, String> {
    let chapters = chapters.collect::<Vec<_>>();
    let total = chapters.len().max(1);
    write_progress(progress_path, "scan", 0, total, "Reading chapter pages");
    chapters
        .iter()
        .enumerate()
        .map(|(index, chapter)| {
            let pages = source.pages(chapter)?;
            write_progress(progress_path, "scan", index + 1, total, &chapter.title);
            Ok((*chapter, pages))
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn append_chapter_images(
    source: &dyn Source,
    chapter: &Chapter,
    pages: &[PageImage],
    folder: &str,
    archive: &mut ZipWriter<File>,
    options: FileOptions,
    progress_path: &Path,
    completed_images: &mut usize,
    total_images: usize,
) -> Result<(), String> {
    for (index, page) in pages.iter().enumerate() {
        let image_data = source.fetch_image(page)?;
        archive
            .start_file(
                format!("{folder}/{:04}{}", index + 1, image_extension(&page.url)),
                options,
            )
            .map_err(|error| error.to_string())?;
        archive
            .write_all(&image_data)
            .map_err(|error| error.to_string())?;
        *completed_images += 1;
        write_progress(
            progress_path,
            "download",
            *completed_images,
            total_images,
            &chapter.title,
        );
    }
    Ok(())
}

fn write_progress(path: &Path, stage: &str, current: usize, total: usize, title: &str) {
    let sanitized_title: String = title
        .chars()
        .map(|character| {
            if matches!(character, '\n' | '\r' | '|') {
                ' '
            } else {
                character
            }
        })
        .collect();
    let _ = fs::write(path, format!("{stage}|{current}|{total}|{sanitized_title}"));
}

fn chapter_filename(chapter_index: usize, manga_name: &str) -> String {
    format!("{} - {}.cbz", chapter_index + 1, safe_filename(manga_name))
}

#[cfg(test)]
mod tests {
    use super::{chapter_filename, download_selection, parse_selection};
    use crate::models::{Chapter, PageImage, SearchResult, SeriesChapters};
    use crate::sources::{SearchOptions, Source};

    /// In-memory source: proves the downloader needs nothing site specific.
    struct MockSource;

    impl Source for MockSource {
        fn id(&self) -> &'static str {
            "mock"
        }
        fn name(&self) -> &'static str {
            "Mock"
        }
        fn search(&self, _: &str, _: &SearchOptions) -> Result<Vec<SearchResult>, String> {
            Ok(Vec::new())
        }
        fn series(&self, series_id: &str) -> Result<SeriesChapters, String> {
            let chapters = (1..=3)
                .map(|n| Chapter {
                    id: format!("c{n}"),
                    title: format!("Chapter {n}"),
                    url: format!("mock://chapter/{n}"),
                    published_at: String::new(),
                })
                .collect();
            Ok(SeriesChapters {
                id: series_id.to_string(),
                title: "Mock Manga".to_string(),
                url: String::new(),
                cover_url: String::new(),
                authors: String::new(),
                tags: String::new(),
                manga_type: String::new(),
                status: String::new(),
                released: String::new(),
                description: String::new(),
                chapters,
            })
        }
        fn pages(&self, chapter: &Chapter) -> Result<Vec<PageImage>, String> {
            Ok((1..=2)
                .map(|n| PageImage {
                    url: format!("{}/{n}.png", chapter.url),
                    referer: None,
                })
                .collect())
        }
        fn fetch_image(&self, page: &PageImage) -> Result<Vec<u8>, String> {
            Ok(page.url.clone().into_bytes())
        }
    }

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("wc_test_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn entry_names(path: &str) -> Vec<String> {
        let mut archive = zip::ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap();
        (0..archive.len())
            .map(|i| archive.by_index(i).unwrap().name().to_string())
            .collect()
    }

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
        // assert_eq!(entry_names(&result.paths[0]), ["0001.png", "0002.png"]);
        assert_eq!(
            entry_names(&result.paths[0]),
            ["Chapter 2/0001.png", "Chapter 2/0002.png"]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

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
        assert_eq!(entry_names(&result.paths[0]).len(), 6);
        // assert_eq!(entry_names(&result.paths[0])[5], "0006.png");

        // merges_chapters_into_one_volume
        let names = entry_names(&result.paths[0]);
        assert_eq!(names.len(), 6);
        assert_eq!(names[0], "0001 - Chapter 1/0001.png");
        assert_eq!(names[5], "0003 - Chapter 3/0002.png");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn generates_manga_filename_without_zero_padding() {
        assert_eq!(chapter_filename(0, "One Piece"), "1 - One Piece.cbz");

        assert_eq!(chapter_filename(1, "One Piece"), "2 - One Piece.cbz");

        assert_eq!(chapter_filename(9, "One Piece"), "10 - One Piece.cbz");

        // // downloads_one_cbz_per_chapter_from_any_source
        // assert_eq!(
        //     entry_names(&result.paths[0]),
        //     ["Chapter 2/0001.png", "Chapter 2/0002.png"]
        // );
        //
        // // merges_chapters_into_one_volume
        // let names = entry_names(&result.paths[0]);
        // assert_eq!(names.len(), 6);
        // assert_eq!(names[0], "0001 - Chapter 1/0001.png");
        // assert_eq!(names[5], "0003 - Chapter 3/0002.png");
    }

    #[test]
    fn parses_single_and_discrete_chapters() {
        assert_eq!(parse_selection("1,3,4", 10).unwrap(), vec![0, 2, 3]);
    }

    #[test]
    fn parses_ranges_and_removes_duplicates() {
        assert_eq!(parse_selection("1-3,3,5", 10).unwrap(), vec![0, 1, 2, 4]);
    }

    #[test]
    fn rejects_chapters_outside_the_list() {
        assert!(parse_selection("0,4", 3).is_err());
        assert!(parse_selection("1-4", 3).is_err());
    }
}
