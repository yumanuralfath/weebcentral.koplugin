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
#[path = "../tests/unit/download/mod.rs"]
mod tests;
