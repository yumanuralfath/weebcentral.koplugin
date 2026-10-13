use regex::Regex;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::models::DownloadResult;
use crate::util::{BASE_URL, USER_AGENT, absolute_url, get_text, safe_filename, validate_id};

pub fn download_chapter(
    chapter_id: &str,
    title: &str,
    output_dir: &str,
) -> Result<DownloadResult, String> {
    validate_id(chapter_id)?;
    let output_dir = Path::new(output_dir);
    fs::create_dir_all(output_dir).map_err(|error| error.to_string())?;

    let cbz_path = output_dir.join(format!("{}.cbz", safe_filename(title)));
    if cbz_path.is_file() {
        return Ok(DownloadResult {
            path: cbz_path.to_string_lossy().into_owned(),
            already_exists: true,
        });
    }

    let chapter_html = get_text(&format!("{BASE_URL}/chapters/{chapter_id}"))?;
    let first_page_url = find_first_page_url(&chapter_html)?;
    let (image_prefix, number_width, extension) = parse_page_url(&first_page_url)?;
    let page_count = page_count(&chapter_html);
    let temporary_path = cbz_path.with_extension("cbz.part");

    if let Err(error) = write_cbz(
        chapter_id,
        &temporary_path,
        &image_prefix,
        number_width,
        &extension,
        page_count,
    ) {
        let _ = fs::remove_file(&temporary_path);
        return Err(error);
    }
    fs::rename(&temporary_path, &cbz_path).map_err(|error| error.to_string())?;

    Ok(DownloadResult {
        path: cbz_path.to_string_lossy().into_owned(),
        already_exists: false,
    })
}

fn find_first_page_url(chapter_html: &str) -> Result<String, String> {
    let document = scraper::Html::parse_document(chapter_html);
    let preload_selector =
        scraper::Selector::parse("link[rel='preload']").expect("valid image preload selector");
    document
        .select(&preload_selector)
        .next()
        .and_then(|link| link.value().attr("href"))
        .map(absolute_url)
        .ok_or_else(|| "Could not find the first page image URL".to_string())
}

fn parse_page_url(url: &str) -> Result<(String, usize, String), String> {
    let pattern = Regex::new(r"^(?P<prefix>.+-)(?P<number>\d+)(?P<extension>\.\w+)(?:\?.*)?$")
        .map_err(|error| error.to_string())?;
    let captures = pattern
        .captures(url)
        .ok_or_else(|| format!("Unrecognized page image URL: {url}"))?;

    Ok((
        captures.name("prefix").unwrap().as_str().to_string(),
        captures.name("number").unwrap().as_str().len(),
        captures.name("extension").unwrap().as_str().to_string(),
    ))
}

fn page_count(chapter_html: &str) -> usize {
    let pattern = Regex::new(r"page === (\d+) \?").expect("valid page count pattern");
    pattern
        .captures_iter(chapter_html)
        .filter_map(|capture| capture[1].parse::<usize>().ok())
        .max()
        .unwrap_or(1)
}

fn write_cbz(
    chapter_id: &str,
    cbz_path: &Path,
    image_prefix: &str,
    number_width: usize,
    extension: &str,
    page_count: usize,
) -> Result<(), String> {
    let file = File::create(cbz_path).map_err(|error| error.to_string())?;
    let mut archive = ZipWriter::new(file);
    let options = FileOptions::default().compression_method(CompressionMethod::Deflated);
    let referer = format!("{BASE_URL}/chapters/{chapter_id}");

    for page_number in 1..=page_count {
        let image_url = format!("{image_prefix}{page_number:0number_width$}{extension}");
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
            .start_file(format!("{page_number:03}{extension}"), options)
            .map_err(|error| error.to_string())?;
        archive
            .write_all(&image_data)
            .map_err(|error| error.to_string())?;
    }

    archive.finish().map_err(|error| error.to_string())?;
    Ok(())
}
