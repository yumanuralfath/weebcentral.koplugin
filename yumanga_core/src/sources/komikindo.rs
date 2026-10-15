//! komikindo.ch
use scraper::{Html, Selector};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

use super::{SearchOptions, Source};
use crate::models::{Chapter, PageImage, SearchResult, SeriesChapters};
use crate::util::{
    USER_AGENT, absolute_url, get_text, id_from_href, normalize_whitespace, validate_id,
};

const BASE_URL: &str = "https://komikindo.ch";

pub struct KomikIndo;

impl Source for KomikIndo {
    fn id(&self) -> &'static str {
        "komikindo"
    }

    fn name(&self) -> &'static str {
        "KomikIndo"
    }

    fn search(&self, keyword: &str, options: &SearchOptions) -> Result<Vec<SearchResult>, String> {
        search(keyword, options.cover_cache_dir, options.with_details)
    }

    fn series(&self, series_id: &str) -> Result<SeriesChapters, String> {
        load_series(series_id)
    }

    fn pages(&self, chapter: &Chapter) -> Result<Vec<PageImage>, String> {
        let html = get_text(&chapter.url)?;
        parse_pages(chapter, &html)
    }
}

fn abs(value: &str) -> String {
    absolute_url(BASE_URL, value)
}

pub fn search(
    keyword: &str,
    cover_cache_dir: &str,
    with_details: bool,
) -> Result<Vec<SearchResult>, String> {
    let response = ureq::get(BASE_URL)
        .query("s", keyword)
        .set("User-Agent", USER_AGENT)
        .call()
        .map_err(|error| error.to_string())?;
    let html = crate::util::read_response_text(response)?;
    let document = Html::parse_document(&html);
    let mut results = parse_search_results(&document);
    if !cover_cache_dir.is_empty() {
        fs::create_dir_all(cover_cache_dir).map_err(|error| error.to_string())?;
    }
    for result in &mut results {
        result.cover_path = cache_cover(&result.id, &result.cover_url, Path::new(cover_cache_dir));
        if with_details {
            if let Ok(html) = get_text(&result.url) {
                let metadata = parse_metadata(&Html::parse_document(&html));
                result.authors = metadata.authors;
                result.tags = metadata.tags;
                result.manga_type = metadata.manga_type;
                result.status = metadata.status;
                result.released = metadata.released;
            }
        }
    }
    Ok(results)
}

fn parse_search_results(document: &Html) -> Vec<SearchResult> {
    let result_selector = Selector::parse(".listupd .animepost .animposx > a[itemprop='url']")
        .expect("valid search result selector");
    let image_selector = Selector::parse("img").expect("valid cover selector");
    document
        .select(&result_selector)
        .filter_map(|item| {
            let href = item.value().attr("href").unwrap_or_default();
            let id = id_from_href(href, "/komik/");
            let title = item
                .value()
                .attr("title")
                .or_else(|| item.value().attr("alt"))
                .map(|title| title.trim_start_matches("Komik ").to_string())
                .filter(|title| !title.is_empty())
                .or_else(|| {
                    item.select(&image_selector)
                        .next()
                        .and_then(|image| image.value().attr("title"))
                        .map(|title| title.trim_start_matches("Komik ").to_string())
                })
                .unwrap_or_default();
            if id.is_empty() || title.is_empty() {
                return None;
            }
            let cover_url = item
                .select(&image_selector)
                .next()
                .and_then(|image| {
                    image
                        .value()
                        .attr("data-src")
                        .or_else(|| image.value().attr("src"))
                })
                .map(abs)
                .unwrap_or_default();
            Some(SearchResult {
                id: id.to_string(),
                title,
                url: abs(href),
                cover_url,
                cover_path: String::new(),
                authors: String::new(),
                tags: String::new(),
                manga_type: String::new(),
                status: String::new(),
                released: String::new(),
            })
        })
        .collect()
}

#[derive(Default)]
struct SeriesMeta {
    authors: String,
    tags: String,
    manga_type: String,
    status: String,
    released: String,
}

fn parse_metadata(document: &Html) -> SeriesMeta {
    let metadata_selector = Selector::parse(".infox .spe span").expect("valid metadata selector");
    let label_selector = Selector::parse("b").expect("valid metadata label selector");
    let mut metadata = SeriesMeta::default();
    for item in document.select(&metadata_selector) {
        let Some(label) = item.select(&label_selector).next() else {
            continue;
        };
        let label = normalize_whitespace(label.text().collect::<Vec<_>>().join(" "))
            .trim_end_matches(':')
            .to_ascii_lowercase();
        let value = normalize_whitespace(item.text().collect::<Vec<_>>().join(" "));
        let value = value
            .split_once(':')
            .map(|(_, value)| value.trim().to_string())
            .unwrap_or_default();
        match label.as_str() {
            "pengarang" | "author" | "author(s)" | "ilustrator" | "artist" => {
                append_value(&mut metadata.authors, value)
            }
            "tema" | "genre" | "genres" => append_value(&mut metadata.tags, value),
            "jenis komik" | "type" => metadata.manga_type = value,
            "status" => metadata.status = value,
            "tahun rilis" | "released" | "year" => metadata.released = value,
            _ => {}
        }
    }
    metadata
}

fn append_value(target: &mut String, value: String) {
    if value.is_empty() {
        return;
    }
    if !target.is_empty() {
        target.push_str(", ");
    }
    target.push_str(&value);
}

pub fn load_series(series_id: &str) -> Result<SeriesChapters, String> {
    validate_id(series_id)?;
    let series_url = format!("{BASE_URL}/komik/{series_id}/");
    let document = Html::parse_document(&get_text(&series_url)?);
    let title_selector = Selector::parse("h1.entry-title").expect("valid title selector");
    let title = document
        .select(&title_selector)
        .next()
        .map(|element| normalize_whitespace(element.text().collect::<Vec<_>>().join(" ")))
        .unwrap_or_default()
        .trim_start_matches("Komik ")
        .to_string();
    let cover_selector = Selector::parse(".thumb img").expect("valid series cover selector");
    let cover_url = document
        .select(&cover_selector)
        .next()
        .and_then(|image| {
            image
                .value()
                .attr("data-src")
                .or_else(|| image.value().attr("src"))
        })
        .map(abs)
        .unwrap_or_default();
    let metadata = parse_metadata(&document);
    let description_selector =
        Selector::parse(".entry-content p").expect("valid description selector");
    let description = document
        .select(&description_selector)
        .map(|paragraph| normalize_whitespace(paragraph.text().collect::<Vec<_>>().join(" ")))
        .find(|text| !text.is_empty())
        .unwrap_or_default();
    let chapters = parse_chapters(&document);

    Ok(SeriesChapters {
        id: series_id.to_string(),
        title,
        url: series_url,
        cover_url,
        authors: metadata.authors,
        tags: metadata.tags,
        manga_type: metadata.manga_type,
        status: metadata.status,
        released: metadata.released,
        description,
        chapters,
    })
}

fn parse_chapters(document: &Html) -> Vec<Chapter> {
    let chapter_selector = Selector::parse("#chapter_list li").expect("valid chapter selector");
    let link_selector = Selector::parse("a").expect("valid chapter link selector");
    let date_selector = Selector::parse(".dt").expect("valid chapter date selector");
    let mut chapters = Vec::new();
    for item in document.select(&chapter_selector) {
        let Some(link) = item.select(&link_selector).next() else {
            continue;
        };
        let href = link.value().attr("href").unwrap_or_default();
        let path = href.strip_prefix(BASE_URL).unwrap_or(href);
        let id = path
            .split(['?', '#'])
            .next()
            .unwrap_or_default()
            .trim_matches('/');
        if id.is_empty() {
            continue;
        }
        let chapter_title = normalize_whitespace(link.text().collect::<Vec<_>>().join(" "));
        let published_at = item
            .select(&date_selector)
            .next()
            .map(|date| normalize_whitespace(date.text().collect::<Vec<_>>().join(" ")))
            .unwrap_or_default();
        chapters.push(Chapter {
            id: id.to_string(),
            title: chapter_title,
            url: abs(href),
            published_at,
        });
    }
    chapters.reverse();
    chapters
}

fn parse_pages(chapter: &Chapter, chapter_html: &str) -> Result<Vec<PageImage>, String> {
    let document = Html::parse_document(chapter_html);
    let image_selector = Selector::parse("#Baca_Komik img").expect("valid reader image selector");
    let mut seen = HashSet::new();
    let pages = document
        .select(&image_selector)
        .filter_map(|image| {
            image
                .value()
                .attr("data-src")
                .or_else(|| image.value().attr("src"))
                .map(abs)
        })
        .filter(|url| !url.is_empty() && seen.insert(url.clone()))
        .map(|url| PageImage {
            url,
            referer: Some(chapter.url.clone()),
        })
        .collect::<Vec<_>>();
    if pages.is_empty() {
        return Err("Could not find chapter page images".to_string());
    }
    Ok(pages)
}

fn cache_cover(id: &str, cover_url: &str, cache_dir: &Path) -> String {
    if cover_url.is_empty() || cache_dir.as_os_str().is_empty() {
        return String::new();
    }
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
    if fs::metadata(&path)
        .map(|meta| meta.len() > 0)
        .unwrap_or(false)
    {
        return path.to_string_lossy().into_owned();
    }
    let response = match ureq::get(cover_url).set("User-Agent", USER_AGENT).call() {
        Ok(response) => response,
        Err(_) => return String::new(),
    };
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

#[cfg(test)]
mod tests {
    use super::{Chapter, Html, parse_chapters, parse_metadata, parse_pages, parse_search_results};

    #[test]
    fn parses_search_results_and_series_metadata() {
        let html = r#"<div class="listupd"><div class="animepost"><div class="animposx"><a href="https://komikindo.ch/komik/846048-eleceed/" itemprop="url" title="Komik Eleceed"><img src="/cover.jpg"></a></div></div></div>
            <div class="infox"><div class="spe"><span><b>Status:</b> Berjalan</span><span><b>Pengarang:</b> Son Jae-Ho</span><span><b>Ilustrator:</b> Zhena</span><span><b>Tema:</b> Animals</span><span><b>Jenis Komik:</b> Manhwa</span></div></div>"#;
        let document = Html::parse_document(html);
        let results = parse_search_results(&document);
        assert_eq!(results[0].id, "846048-eleceed");
        assert_eq!(results[0].title, "Eleceed");
        assert_eq!(results[0].cover_url, "https://komikindo.ch/cover.jpg");

        let metadata = parse_metadata(&document);
        assert_eq!(metadata.authors, "Son Jae-Ho, Zhena");
        assert_eq!(metadata.tags, "Animals");
        assert_eq!(metadata.status, "Berjalan");
        assert_eq!(metadata.manga_type, "Manhwa");
    }

    #[test]
    fn reads_reader_images_in_order_without_collecting_ads() {
        let chapter = Chapter {
            id: "eleceed-chapter-1".into(),
            title: "Chapter 1".into(),
            url: "https://komikindo.ch/eleceed-chapter-1/".into(),
            published_at: String::new(),
        };
        let html = r#"<img src="https://example.com/ad.jpg"><div id="Baca_Komik"><img src="//cdn.example/001.webp"><img src="//cdn.example/002.webp"><img src="//cdn.example/002.webp"></div>"#;
        let pages = parse_pages(&chapter, html).unwrap();
        assert_eq!(pages.len(), 2);
        assert_eq!(pages[0].url, "https://cdn.example/001.webp");
        assert_eq!(pages[1].url, "https://cdn.example/002.webp");
        assert_eq!(pages[0].referer.as_deref(), Some(chapter.url.as_str()));
    }

    #[test]
    fn extracts_chapter_slugs_and_returns_oldest_first() {
        let html = r#"<div id="chapter_list"><ul>
            <li><span class="lchx"><a href="https://komikindo.ch/eleceed-chapter-422/">Chapter 422</a></span><span class="dt">1 hari lalu</span></li>
            <li><span class="lchx"><a href="https://komikindo.ch/eleceed-chapter-421/">Chapter 421</a></span><span class="dt">1 minggu lalu</span></li>
        </ul></div>"#;
        let chapters = parse_chapters(&Html::parse_document(html));
        assert_eq!(chapters.len(), 2);
        assert_eq!(chapters[0].id, "eleceed-chapter-421");
        assert_eq!(chapters[0].title, "Chapter 421");
        assert_eq!(chapters[1].id, "eleceed-chapter-422");
    }

    #[test]
    fn missing_reader_images_is_an_error() {
        let chapter = Chapter {
            id: "eleceed-chapter-1".into(),
            title: "Chapter 1".into(),
            url: String::new(),
            published_at: String::new(),
        };
        assert!(parse_pages(&chapter, "<html></html>").is_err());
    }
}
