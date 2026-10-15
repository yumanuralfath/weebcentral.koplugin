//! mangaku.lol
use super::{SearchOptions, Source};
use crate::models::{Chapter, PageImage, SearchResult, SeriesChapters};
use crate::util::get_text;

const BASE_URL: &str = "https://mangaku.lol";

pub struct Mangaku;

impl Source for Mangaku {
    fn id(&self) -> &'static str {
        "mangaku"
    }

    fn name(&self) -> &'static str {
        "Mangaku"
    }

    fn search(&self, keyword: &str, options: &SearchOptions) -> Result<Vec<SearchResult>, String> {
        super::westmanga::search_for_site(
            BASE_URL,
            keyword,
            options.cover_cache_dir,
            options.with_details,
        )
    }

    fn series(&self, series_id: &str) -> Result<SeriesChapters, String> {
        super::westmanga::load_series_for_site(BASE_URL, series_id)
    }

    fn pages(&self, chapter: &Chapter) -> Result<Vec<PageImage>, String> {
        let html = get_text(&chapter.url)?;
        super::westmanga::parse_pages_for_site(BASE_URL, chapter, &html)
    }
}

#[cfg(test)]
#[path = "../../tests/unit/mangaku/mod.rs"]
mod tests;
