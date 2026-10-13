use regex::Regex;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::models::{Chapter, DownloadResult};
use crate::util::{BASE_URL, USER_AGENT, absolute_url, get_text, safe_filename, validate_id};
use crate::weebcentral::load_chapters;

pub fn download_selection(
    series_id: &str,
    selection: &str,
    volume_name: &str,
    output_dir: &str,
    progress_path: &str,
) -> Result<DownloadResult, String> {
    validate_id(series_id)?;
    let series = load_chapters(series_id)?;
    let selected_indices = parse_selection(selection, series.chapters.len())?;
    let output_dir = Path::new(output_dir);
    fs::create_dir_all(output_dir).map_err(|error| error.to_string())?;
    if let Some(progress_directory) = Path::new(progress_path).parent() {
        fs::create_dir_all(progress_directory).map_err(|error| error.to_string())?;
    }

    let result = if volume_name.trim().is_empty() {
        download_individual_chapters(
            &series.chapters,
            &selected_indices,
            output_dir,
            Path::new(progress_path),
        )
    } else {
        download_merged_volume(
            &series.chapters,
            &selected_indices,
            volume_name,
            output_dir,
            Path::new(progress_path),
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
    chapters: &[Chapter],
    selected_indices: &[usize],
    output_dir: &Path,
    progress_path: &Path,
) -> Result<DownloadResult, String> {
    let mut paths = Vec::new();
    let mut skipped = 0;

    for (position, chapter_index) in selected_indices.iter().enumerate() {
        let chapter = &chapters[*chapter_index];
        write_progress(
            progress_path,
            position,
            selected_indices.len(),
            &chapter.title,
        );
        let path = output_dir.join(format!(
            "{:03}-{}.cbz",
            chapter_index + 1,
            safe_filename(&chapter.title)
        ));
        if path.is_file() {
            skipped += 1;
            paths.push(path.to_string_lossy().into_owned());
            continue;
        }

        let temporary_path = path.with_extension("cbz.part");
        if let Err(error) = write_chapter_cbz(chapter, &temporary_path) {
            let _ = fs::remove_file(&temporary_path);
            return Err(error);
        }
        fs::rename(&temporary_path, &path).map_err(|error| error.to_string())?;
        paths.push(path.to_string_lossy().into_owned());
    }
    write_progress(
        progress_path,
        selected_indices.len(),
        selected_indices.len(),
        "Complete",
    );

    Ok(DownloadResult { paths, skipped })
}

fn download_merged_volume(
    chapters: &[Chapter],
    selected_indices: &[usize],
    volume_name: &str,
    output_dir: &Path,
    progress_path: &Path,
) -> Result<DownloadResult, String> {
    let path = output_dir.join(format!("{}.cbz", safe_filename(volume_name)));
    if path.is_file() {
        return Ok(DownloadResult {
            paths: vec![path.to_string_lossy().into_owned()],
            skipped: 1,
        });
    }

    let temporary_path = path.with_extension("cbz.part");
    let result = write_volume_cbz(chapters, selected_indices, &temporary_path, progress_path);
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
    chapters: &[Chapter],
    selected_indices: &[usize],
    path: &Path,
    progress_path: &Path,
) -> Result<(), String> {
    let file = File::create(path).map_err(|error| error.to_string())?;
    let mut archive = ZipWriter::new(file);
    let options = FileOptions::default().compression_method(CompressionMethod::Deflated);
    let mut page_number = 1;

    for (position, chapter_index) in selected_indices.iter().enumerate() {
        let chapter = &chapters[*chapter_index];
        write_progress(
            progress_path,
            position,
            selected_indices.len(),
            &chapter.title,
        );
        append_chapter_images(chapter, &mut archive, options, &mut page_number)?;
    }
    archive.finish().map_err(|error| error.to_string())?;
    write_progress(
        progress_path,
        selected_indices.len(),
        selected_indices.len(),
        "Complete",
    );
    Ok(())
}

fn write_chapter_cbz(chapter: &Chapter, path: &Path) -> Result<(), String> {
    let file = File::create(path).map_err(|error| error.to_string())?;
    let mut archive = ZipWriter::new(file);
    let options = FileOptions::default().compression_method(CompressionMethod::Deflated);
    let mut page_number = 1;
    append_chapter_images(chapter, &mut archive, options, &mut page_number)?;
    archive.finish().map_err(|error| error.to_string())?;
    Ok(())
}

fn append_chapter_images(
    chapter: &Chapter,
    archive: &mut ZipWriter<File>,
    options: FileOptions,
    page_number: &mut usize,
) -> Result<(), String> {
    let chapter_html = get_text(&chapter.url)?;
    let (prefix, number_width, extension) = page_image_pattern(&chapter_html)?;
    let page_count = get_page_count(&chapter_html);
    let referer = format!("{BASE_URL}/chapters/{}", chapter.id);

    for image_index in 1..=page_count {
        let image_url = format!("{prefix}{image_index:0number_width$}{extension}");
        let response = ureq::get(&image_url)
            .set("User-Agent", USER_AGENT)
            .set("Referer", &referer)
            .call()
            .map_err(|error| error.to_string())?;
        let mut image_data = Vec::new();
        response
            .into_reader()
            .read_to_end(&mut image_data)
            .map_err(|error| error.to_string())?;

        archive
            .start_file(format!("{page_number:04}{extension}"), options)
            .map_err(|error| error.to_string())?;
        archive
            .write_all(&image_data)
            .map_err(|error| error.to_string())?;
        *page_number += 1;
    }
    Ok(())
}

fn page_image_pattern(chapter_html: &str) -> Result<(String, usize, String), String> {
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
    Ok((
        captures.name("prefix").unwrap().as_str().to_string(),
        captures.name("number").unwrap().as_str().len(),
        captures.name("extension").unwrap().as_str().to_string(),
    ))
}

fn get_page_count(chapter_html: &str) -> usize {
    let pattern = Regex::new(r"page === (\d+) \?").expect("valid page count pattern");
    pattern
        .captures_iter(chapter_html)
        .filter_map(|capture| capture[1].parse::<usize>().ok())
        .max()
        .unwrap_or(1)
}

fn write_progress(path: &Path, current: usize, total: usize, title: &str) {
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
    let _ = fs::write(path, format!("{current}|{total}|{sanitized_title}"));
}

#[cfg(test)]
mod tests {
    use super::parse_selection;

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
