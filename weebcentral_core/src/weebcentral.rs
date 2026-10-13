use scraper::{Html, Selector};

use crate::models::{Chapter, SearchResult, SeriesChapters};
use crate::util::{
    BASE_URL, USER_AGENT, absolute_url, get_text, id_from_href, normalize_whitespace,
    read_response_text,
};

const SEARCH_URL: &str = "https://weebcentral.com/search/simple?location=main";

pub fn search(keyword: &str) -> Result<Vec<SearchResult>, String> {
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

        results.push(SearchResult {
            id: id.to_string(),
            title,
            url: absolute_url(href),
            cover_url,
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

    Ok(SeriesChapters { title, chapters })
}
