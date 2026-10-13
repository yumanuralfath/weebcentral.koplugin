use regex::Regex;
use scraper::{Html, Selector};
use serde::Serialize;
use serde_json::json;
use std::env;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipWriter};

const BASE_URL: &str = "https://weebcentral.com";
const SEARCH_URL: &str = "https://weebcentral.com/search/simple?location=main";
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36";

#[derive(Serialize)]
struct SearchResult {
    id: String,
    title: String,
    url: String,
    cover_url: String,
}

#[derive(Serialize)]
struct Chapter {
    id: String,
    title: String,
    url: String,
    published_at: String,
}

#[derive(Serialize)]
struct SeriesChapters {
    title: String,
    chapters: Vec<Chapter>,
}

#[derive(Serialize)]
struct DownloadResult {
    path: String,
    already_exists: bool,
}

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("search") => match args.next() {
            Some(keyword) if !keyword.trim().is_empty() => emit_result(search(&keyword)),
            _ => emit_error("Search keyword is required"),
        },
        Some("chapters") => match args.next() {
            Some(series_id) => emit_result(load_chapters(&series_id)),
            None => emit_error("Series ID is required"),
        },
        Some("download") => match (args.next(), args.next(), args.next()) {
            (Some(chapter_id), Some(title), Some(output_dir)) => {
                emit_result(download_chapter(&chapter_id, &title, &output_dir))
            }
            _ => {
                emit_error("Usage: weebcentral_core_dev download <chapter-id> <title> <output-dir>")
            }
        },
        _ => emit_error("Usage: weebcentral_core_dev search <keyword>"),
    }
}

fn emit_error(error: impl std::fmt::Display) -> ExitCode {
    println!("{}", json!({ "ok": false, "error": error.to_string() }));
    ExitCode::FAILURE
}

fn emit_result<T: Serialize>(result: Result<T, String>) -> ExitCode {
    match result {
        Ok(data) => {
            println!("{}", json!({ "ok": true, "data": data }));
            ExitCode::SUCCESS
        }
        Err(error) => emit_error(error),
    }
}

fn search(keyword: &str) -> Result<Vec<SearchResult>, String> {
    let response = ureq::post(SEARCH_URL)
        .set("User-Agent", USER_AGENT)
        .send_form(&[("text", keyword)])
        .map_err(|error| error.to_string())?;
    let mut html = String::new();
    response
        .into_reader()
        .read_to_string(&mut html)
        .map_err(|error| error.to_string())?;
    let document = Html::parse_document(&html);
    let result_selector = Selector::parse("section#quick-search-result a.join-item")
        .expect("valid search result selector");
    let title_selector = Selector::parse("div.flex-1").expect("valid title selector");
    let picture_selector = Selector::parse("picture").expect("valid picture selector");
    let source_selector =
        Selector::parse("source[type='image/webp']").expect("valid cover source selector");
    let image_selector = Selector::parse("img").expect("valid image selector");

    let mut results = Vec::new();
    for item in document.select(&result_selector) {
        let href = item.value().attr("href").unwrap_or_default();
        let id = id_from_href(href, "/series/");
        let title = item
            .select(&title_selector)
            .next()
            .map(|element| normalize_whitespace(element.text().collect::<Vec<_>>().join(" ")))
            .unwrap_or_default()
            .to_string();

        if id.is_empty() || title.is_empty() {
            continue;
        }

        let cover = item
            .select(&picture_selector)
            .next()
            .and_then(|picture| {
                picture
                    .select(&source_selector)
                    .next()
                    .and_then(|source| source.value().attr("srcset"))
                    .or_else(|| {
                        picture
                            .select(&image_selector)
                            .next()
                            .and_then(|image| image.value().attr("src"))
                    })
            })
            .unwrap_or_default();

        results.push(SearchResult {
            id: id.to_string(),
            title,
            url: absolute_url(href),
            cover_url: absolute_url(cover),
        });
    }

    Ok(results)
}

fn load_chapters(series_id: &str) -> Result<SeriesChapters, String> {
    validate_id(series_id)?;
    let series_url = format!("{BASE_URL}/series/{series_id}");
    let series_html = get_text(&series_url)?;
    let series_document = Html::parse_document(&series_html);
    let title_selector = Selector::parse("h1").expect("valid series title selector");
    let title = series_document
        .select(&title_selector)
        .next()
        .map(|element| normalize_whitespace(element.text().collect::<Vec<_>>().join(" ")))
        .unwrap_or_default()
        .to_string();

    let chapters_url = format!("{series_url}/full-chapter-list");
    let response = ureq::get(&chapters_url)
        .set("User-Agent", USER_AGENT)
        .set("HX-Request", "true")
        .call()
        .map_err(|error| error.to_string())?;
    let chapters_html = read_text(response)?;
    let chapters_document = Html::parse_document(&chapters_html);
    let chapter_selector =
        Selector::parse("a[href^='/chapters/']").expect("valid chapter link selector");
    let chapter_title_selector =
        Selector::parse("span.grow > span:first-child").expect("valid chapter title selector");
    let time_selector = Selector::parse("time").expect("valid published date selector");

    let mut chapters = Vec::new();
    for item in chapters_document.select(&chapter_selector) {
        let href = item.value().attr("href").unwrap_or_default();
        let id = id_from_href(href, "/chapters/");
        if id.is_empty() {
            continue;
        }
        let chapter_title = item
            .select(&chapter_title_selector)
            .next()
            .map(|element| normalize_whitespace(element.text().collect::<Vec<_>>().join(" ")))
            .unwrap_or_else(|| "Unknown chapter".to_string())
            .to_string();
        let published_at = item
            .select(&time_selector)
            .next()
            .and_then(|time| time.value().attr("datetime"))
            .unwrap_or_default()
            .to_string();

        chapters.push(Chapter {
            id: id.to_string(),
            title: chapter_title,
            url: absolute_url(href),
            published_at,
        });
    }
    chapters.reverse();

    Ok(SeriesChapters { title, chapters })
}

fn download_chapter(
    chapter_id: &str,
    title: &str,
    output_dir: &str,
) -> Result<DownloadResult, String> {
    validate_id(chapter_id)?;
    let output_dir = PathBuf::from(output_dir);
    fs::create_dir_all(&output_dir).map_err(|error| error.to_string())?;
    let filename = safe_filename(title);
    let cbz_path = output_dir.join(format!("{filename}.cbz"));
    if cbz_path.is_file() {
        return Ok(DownloadResult {
            path: cbz_path.to_string_lossy().into_owned(),
            already_exists: true,
        });
    }

    let chapter_url = format!("{BASE_URL}/chapters/{chapter_id}");
    let chapter_html = get_text(&chapter_url)?;
    let chapter_document = Html::parse_document(&chapter_html);
    let preload_selector =
        Selector::parse("link[rel='preload']").expect("valid image preload selector");
    let first_page = chapter_document
        .select(&preload_selector)
        .next()
        .and_then(|link| link.value().attr("href"))
        .ok_or_else(|| "Could not find the first page image URL".to_string())?;
    let first_page = absolute_url(first_page);
    let page_pattern = Regex::new(r"^(?P<prefix>.+-)(?P<number>\d+)(?P<extension>\.\w+)(?:\?.*)?$")
        .map_err(|error| error.to_string())?;
    let captures = page_pattern
        .captures(&first_page)
        .ok_or_else(|| format!("Unrecognized page image URL: {first_page}"))?;
    let prefix = captures.name("prefix").unwrap().as_str();
    let number_width = captures.name("number").unwrap().as_str().len();
    let extension = captures.name("extension").unwrap().as_str();
    let page_count_pattern = Regex::new(r"page === (\d+) \?").unwrap();
    let page_count = page_count_pattern
        .captures_iter(&chapter_html)
        .filter_map(|capture| capture[1].parse::<usize>().ok())
        .max()
        .unwrap_or(1);

    let temporary_path = cbz_path.with_extension("cbz.part");
    let result = write_chapter_cbz(
        chapter_id,
        &temporary_path,
        prefix,
        number_width,
        extension,
        page_count,
    );
    if let Err(error) = result {
        let _ = fs::remove_file(&temporary_path);
        return Err(error);
    }
    fs::rename(&temporary_path, &cbz_path).map_err(|error| error.to_string())?;

    Ok(DownloadResult {
        path: cbz_path.to_string_lossy().into_owned(),
        already_exists: false,
    })
}

fn write_chapter_cbz(
    chapter_id: &str,
    cbz_path: &PathBuf,
    prefix: &str,
    number_width: usize,
    extension: &str,
    page_count: usize,
) -> Result<(), String> {
    let file = File::create(cbz_path).map_err(|error| error.to_string())?;
    let mut archive = ZipWriter::new(file);
    let options = FileOptions::default().compression_method(CompressionMethod::Deflated);
    let referer = format!("{BASE_URL}/chapters/{chapter_id}");

    for page in 1..=page_count {
        let image_url = format!("{prefix}{page:0number_width$}{extension}");
        let response = ureq::get(&image_url)
            .set("User-Agent", USER_AGENT)
            .set("Referer", &referer)
            .call()
            .map_err(|error| error.to_string())?;
        let mut bytes = Vec::new();
        response
            .into_reader()
            .read_to_end(&mut bytes)
            .map_err(|error| error.to_string())?;

        archive
            .start_file(format!("{page:03}{extension}"), options)
            .map_err(|error| error.to_string())?;
        archive
            .write_all(&bytes)
            .map_err(|error| error.to_string())?;
    }

    archive.finish().map_err(|error| error.to_string())?;
    Ok(())
}

fn get_text(url: &str) -> Result<String, String> {
    let response = ureq::get(url)
        .set("User-Agent", USER_AGENT)
        .call()
        .map_err(|error| error.to_string())?;
    read_text(response)
}

fn read_text(response: ureq::Response) -> Result<String, String> {
    let mut text = String::new();
    response
        .into_reader()
        .read_to_string(&mut text)
        .map_err(|error| error.to_string())?;
    Ok(text)
}

fn id_from_href<'a>(href: &'a str, route: &str) -> &'a str {
    href.split_once(route)
        .map(|(_, remainder)| remainder)
        .unwrap_or_default()
        .split(|character| character == '/' || character == '?')
        .next()
        .unwrap_or_default()
}

fn normalize_whitespace(value: String) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn validate_id(value: &str) -> Result<(), String> {
    if value.is_empty()
        || !value.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '-' || character == '_'
        })
    {
        return Err("Invalid series or chapter ID".to_string());
    }
    Ok(())
}

fn safe_filename(value: &str) -> String {
    let filename: String = value
        .chars()
        .map(|character| {
            if character.is_control()
                || matches!(
                    character,
                    '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'
                )
            {
                '_'
            } else {
                character
            }
        })
        .collect();
    let filename = filename
        .trim()
        .trim_matches(|character| character == '.' || character == ' ');
    if filename.is_empty() {
        "chapter".to_string()
    } else {
        filename.to_string()
    }
}

fn absolute_url(value: &str) -> String {
    let url = value.split_whitespace().next().unwrap_or_default();
    if url.starts_with("http://") || url.starts_with("https://") {
        url.to_string()
    } else if url.starts_with("//") {
        format!("https:{url}")
    } else if url.starts_with('/') {
        format!("{BASE_URL}{url}")
    } else if url.is_empty() {
        String::new()
    } else {
        format!("{BASE_URL}/{url}")
    }
}
