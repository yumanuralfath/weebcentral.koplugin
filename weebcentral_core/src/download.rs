use regex::Regex;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::models::{Chapter, DownloadResult, SeriesChapters};
use crate::util::{BASE_URL, USER_AGENT, absolute_url, get_text, safe_filename, validate_id};
use crate::weebcentral::load_chapters;

struct ChapterPages {
    prefix: String,
    number_width: usize,
    extension: String,
    count: usize,
    referer: String,
}

pub fn download_selection(
    series_id: &str,
    selection: &str,
    volume_name: &str,
    output_dir: &str,
    progress_path: &str,
) -> Result<DownloadResult, String> {
    validate_id(series_id)?;
    let progress_path = Path::new(progress_path);
    if let Some(progress_directory) = progress_path.parent() {
        fs::create_dir_all(progress_directory).map_err(|error| error.to_string())?;
    }
    write_progress(progress_path, "scan", 0, 1, "Loading chapter list");
    let series = load_chapters(series_id)?;
    let selected_indices = parse_selection(selection, series.chapters.len())?;
    let output_dir = Path::new(output_dir);
    fs::create_dir_all(output_dir).map_err(|error| error.to_string())?;

    let result = if volume_name.trim().is_empty() {
        download_individual_chapters(
            &series,
            &series.chapters,
            &selected_indices,
            output_dir,
            progress_path,
        )
    } else {
        download_merged_volume(
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

    let prepared = prepare_chapters(jobs.iter().map(|(chapter, _)| *chapter), progress_path)?;
    let total_images = prepared
        .iter()
        .map(|(_, pages)| pages.count)
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
    let prepared = prepare_chapters(selected_chapters.iter().copied(), progress_path)?;
    let total_images = prepared
        .iter()
        .map(|(_, pages)| pages.count)
        .sum::<usize>()
        .max(1);
    write_progress(
        progress_path,
        "download",
        0,
        total_images,
        "Preparing download",
    );
    let result = write_volume_cbz(&prepared, &temporary_path, progress_path, total_images);
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
    prepared: &[(&Chapter, ChapterPages)],
    path: &Path,
    progress_path: &Path,
    total_images: usize,
) -> Result<(), String> {
    let file = File::create(path).map_err(|error| error.to_string())?;
    let mut archive = ZipWriter::new(file);
    let options = FileOptions::default().compression_method(CompressionMethod::Deflated);
    let mut page_number = 1;

    let mut completed_images = 0;
    for (chapter, pages) in prepared {
        append_chapter_images(
            chapter,
            pages,
            &mut archive,
            options,
            &mut page_number,
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
    chapter: &Chapter,
    pages: &ChapterPages,
    path: &Path,
    progress_path: &Path,
    completed_images: &mut usize,
    total_images: usize,
) -> Result<(), String> {
    let file = File::create(path).map_err(|error| error.to_string())?;
    let mut archive = ZipWriter::new(file);
    let options = FileOptions::default().compression_method(CompressionMethod::Deflated);
    let mut page_number = 1;
    append_chapter_images(
        chapter,
        pages,
        &mut archive,
        options,
        &mut page_number,
        progress_path,
        completed_images,
        total_images,
    )?;
    archive.finish().map_err(|error| error.to_string())?;
    Ok(())
}

fn prepare_chapters<'a>(
    chapters: impl Iterator<Item = &'a Chapter>,
    progress_path: &Path,
) -> Result<Vec<(&'a Chapter, ChapterPages)>, String> {
    let chapters = chapters.collect::<Vec<_>>();
    let total = chapters.len().max(1);
    write_progress(progress_path, "scan", 0, total, "Reading chapter pages");
    chapters
        .iter()
        .enumerate()
        .map(|(index, chapter)| {
            let html = get_text(&chapter.url)?;
            let pages = page_manifest(chapter, &html)?;
            write_progress(progress_path, "scan", index + 1, total, &chapter.title);
            Ok((*chapter, pages))
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn append_chapter_images(
    chapter: &Chapter,
    pages: &ChapterPages,
    archive: &mut ZipWriter<File>,
    options: FileOptions,
    page_number: &mut usize,
    progress_path: &Path,
    completed_images: &mut usize,
    total_images: usize,
) -> Result<(), String> {
    for image_index in 1..=pages.count {
        let image_url = format!(
            "{}{image_index:0width$}{}",
            pages.prefix,
            pages.extension,
            width = pages.number_width
        );
        let response = ureq::get(&image_url)
            .set("User-Agent", USER_AGENT)
            .set("Referer", &pages.referer)
            .call()
            .map_err(|error| error.to_string())?;
        let mut image_data = Vec::new();
        response
            .into_reader()
            .read_to_end(&mut image_data)
            .map_err(|error| error.to_string())?;

        archive
            .start_file(format!("{page_number:04}{}", pages.extension), options)
            .map_err(|error| error.to_string())?;
        archive
            .write_all(&image_data)
            .map_err(|error| error.to_string())?;
        *page_number += 1;
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

fn page_manifest(chapter: &Chapter, chapter_html: &str) -> Result<ChapterPages, String> {
    let document = scraper::Html::parse_document(chapter_html);
    let selector =
        scraper::Selector::parse("link[rel='preload']").expect("valid first page image selector");
    let first_page = document
        .select(&selector)
        .next()
        .and_then(|link| link.value().attr("href"))
        .map(absolute_url)
        .ok_or_else(|| "Could not find the first page image URL".to_string())?;
    let pattern = Regex::new(r"^(?P<prefix>.+-)(?P<number>\d+)(?P<extension>\.\w+)(?:\?.*)?$")
        .map_err(|error| error.to_string())?;
    let captures = pattern
        .captures(&first_page)
        .ok_or_else(|| format!("Unrecognized page image URL: {first_page}"))?;
    Ok(ChapterPages {
        prefix: captures.name("prefix").unwrap().as_str().to_string(),
        number_width: captures.name("number").unwrap().as_str().len(),
        extension: captures.name("extension").unwrap().as_str().to_string(),
        count: get_page_count(chapter_html),
        referer: format!("{BASE_URL}/chapters/{}", chapter.id),
    })
}

fn get_page_count(chapter_html: &str) -> usize {
    let pattern = Regex::new(r"page === (\d+) \?").expect("valid page count pattern");
    pattern
        .captures_iter(chapter_html)
        .filter_map(|capture| capture[1].parse::<usize>().ok())
        .max()
        .unwrap_or(1)
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
    use super::{chapter_filename, parse_selection};

    #[test]
    fn generates_manga_filename_without_zero_padding() {
        assert_eq!(chapter_filename(0, "One Piece"), "1 - One Piece.cbz");

        assert_eq!(chapter_filename(1, "One Piece"), "2 - One Piece.cbz");

        assert_eq!(chapter_filename(9, "One Piece"), "10 - One Piece.cbz");
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
