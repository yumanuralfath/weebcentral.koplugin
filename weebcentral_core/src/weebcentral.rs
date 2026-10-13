use scraper::{Html, Selector};
use std::fs;
use std::path::Path;

use crate::models::{Chapter, SearchResult, SeriesChapters};
use crate::util::{
    BASE_URL, USER_AGENT, absolute_url, get_text, id_from_href, normalize_whitespace,
    read_response_text,
};

const SEARCH_URL: &str = "https://weebcentral.com/search/simple?location=main";

pub fn search(keyword: &str, cover_cache_dir: &str) -> Result<Vec<SearchResult>, String> {
    let response = ureq::post(SEARCH_URL)
        .set("User-Agent", USER_AGENT)
        .send_form(&[("text", keyword)])
        .map_err(|error| error.to_string())?;
    let html = read_response_text(response)?;
    let document = Html::parse_document(&html);
    let result_selector = Selector::parse("section#quick-search-result a.join-item")
        .expect("valid search result selector");
    let title_selector = Selector::parse("div.flex-1").expect("valid title selector");
    let picture_selector = Selector::parse("picture").expect("valid picture selector");
    let webp_selector =
        Selector::parse("source[type='image/webp']").expect("valid cover source selector");
    let image_selector = Selector::parse("img").expect("valid image selector");
    if !cover_cache_dir.is_empty() {
        fs::create_dir_all(cover_cache_dir).map_err(|error| error.to_string())?;
    }

    let mut results = Vec::new();
    for item in document.select(&result_selector) {
        let href = item.value().attr("href").unwrap_or_default();
        let id = id_from_href(href, "/series/");
        let title = item
            .select(&title_selector)
            .next()
            .map(|element| normalize_whitespace(element.text().collect::<Vec<_>>().join(" ")))
            .unwrap_or_default();
        if id.is_empty() || title.is_empty() {
            continue;
        }

        let cover_url = item
            .select(&picture_selector)
            .next()
            .and_then(|picture| {
                picture
                    .select(&webp_selector)
                    .next()
                    .and_then(|source| source.value().attr("srcset"))
                    .or_else(|| {
                        picture
                            .select(&image_selector)
                            .next()
                            .and_then(|image| image.value().attr("src"))
                    })
            })
            .map(absolute_url)
            .unwrap_or_default();
        let cover_path = cache_cover(id, &cover_url, Path::new(cover_cache_dir));

        results.push(SearchResult {
            id: id.to_string(),
            title,
            url: absolute_url(href),
            cover_url,
            cover_path,
        });
    }

    Ok(results)
}

pub fn load_chapters(series_id: &str) -> Result<SeriesChapters, String> {
    crate::util::validate_id(series_id)?;
    let series_url = format!("{BASE_URL}/series/{series_id}");
    let series_document = Html::parse_document(&get_text(&series_url)?);
    let title_selector = Selector::parse("h1").expect("valid series title selector");
    let title = series_document
        .select(&title_selector)
        .next()
        .map(|element| normalize_whitespace(element.text().collect::<Vec<_>>().join(" ")))
        .unwrap_or_default();
    let cover_selector = Selector::parse("picture").expect("valid series cover selector");
    let webp_selector =
        Selector::parse("source[type='image/webp']").expect("valid cover source selector");
    let image_selector = Selector::parse("img").expect("valid image selector");
    let cover_url = series_document
        .select(&cover_selector)
        .next()
        .and_then(|picture| {
            picture
                .select(&webp_selector)
                .next()
                .and_then(|source| source.value().attr("srcset"))
                .or_else(|| {
                    picture
                        .select(&image_selector)
                        .next()
                        .and_then(|image| image.value().attr("src"))
                })
        })
        .map(absolute_url)
        .unwrap_or_default();
    let authors = list_item_value(&series_document, "Author(s)");
    let tags = list_item_value(&series_document, "Tags(s)");
    let manga_type = list_item_value(&series_document, "Type");
    let status = list_item_value(&series_document, "Status");
    let description = list_item_value(&series_document, "Description");

    let chapters_url = format!("{series_url}/full-chapter-list");
    let response = ureq::get(&chapters_url)
        .set("User-Agent", USER_AGENT)
        .set("HX-Request", "true")
        .call()
        .map_err(|error| error.to_string())?;
    let chapters_document = Html::parse_document(&read_response_text(response)?);
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
        let title = item
            .select(&chapter_title_selector)
            .next()
            .map(|element| normalize_whitespace(element.text().collect::<Vec<_>>().join(" ")))
            .unwrap_or_else(|| "Unknown chapter".to_string());
        let published_at = item
            .select(&time_selector)
            .next()
            .and_then(|time| time.value().attr("datetime"))
            .unwrap_or_default()
            .to_string();

        chapters.push(Chapter {
            id: id.to_string(),
            title,
            url: absolute_url(href),
            published_at,
        });
    }
    chapters.reverse();

    Ok(SeriesChapters {
        id: series_id.to_string(),
        title,
        url: series_url,
        cover_url,
        authors,
        tags,
        manga_type,
        status,
        description,
        chapters,
    })
}

fn list_item_value(document: &Html, label: &str) -> String {
    let item_selector = Selector::parse("li").expect("valid metadata list selector");
    let strong_selector = Selector::parse("strong").expect("valid metadata label selector");
    let link_selector = Selector::parse("a").expect("valid metadata link selector");
    let span_selector = Selector::parse("span").expect("valid metadata value selector");
    let paragraph_selector = Selector::parse("p").expect("valid description selector");

    let Some(item) = document.select(&item_selector).find(|item| {
        item.select(&strong_selector)
            .next()
            .map(|strong| normalize_whitespace(strong.text().collect::<Vec<_>>().join(" ")))
            .map(|text| text.trim_end_matches(':') == label.trim_end_matches(':'))
            .unwrap_or(false)
    }) else {
        return String::new();
    };

    if label == "Description" {
        return item
            .select(&paragraph_selector)
            .next()
            .map(|paragraph| normalize_whitespace(paragraph.text().collect::<Vec<_>>().join(" ")))
            .unwrap_or_default();
    }
    let links = item
        .select(&link_selector)
        .map(|link| normalize_whitespace(link.text().collect::<Vec<_>>().join(" ")))
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    if !links.is_empty() {
        return links.join(", ");
    }
    item.select(&span_selector)
        .next()
        .map(|span| normalize_whitespace(span.text().collect::<Vec<_>>().join(" ")))
        .unwrap_or_default()
}

fn cache_cover(id: &str, cover_url: &str, cache_dir: &Path) -> String {
    if cover_url.is_empty() || cache_dir.as_os_str().is_empty() {
        return String::new();
    }
    let response = match ureq::get(cover_url).set("User-Agent", USER_AGENT).call() {
        Ok(response) => response,
        Err(_) => return String::new(),
    };
    let extension = cover_url
        .split('?')
        .next()
        .and_then(|url| url.rsplit('.').next())
        .filter(|extension| {
            extension.len() <= 5
                && extension
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric())
        })
        .unwrap_or("img");
    let path = cache_dir.join(format!("{id}.{extension}"));
    let mut reader = response.into_reader();
    let mut output = match fs::File::create(&path) {
        Ok(output) => output,
        Err(_) => return String::new(),
    };
    if std::io::copy(&mut reader, &mut output).is_err() {
        let _ = fs::remove_file(&path);
        return String::new();
    }
    path.to_string_lossy().into_owned()
}
