//! Manga sources. Every website the plugin can read from implements [`Source`].
//!
//! Built-in sites ship as Rhai scripts under `YuManga.koplugin/extensions/`.
//! Drop another `*.rhai` there to add a source without rebuilding the core.

mod rhai_host;

use crate::models::{Chapter, PageImage, SearchResult, SeriesChapters};
use crate::util;
use std::sync::OnceLock;

pub struct SearchOptions<'a> {
    /// Directory where cover images are cached ("" disables covers).
    pub cover_cache_dir: &'a str,
    /// Fetch per-series details (authors, tags...) for every result.
    pub with_details: bool,
}

pub trait Source: Send + Sync {
    /// Stable identifier used on the command line, e.g. "weebcentral".
    fn id(&self) -> &str;

    /// Human readable name shown in the UI.
    fn name(&self) -> &str;

    fn search(&self, keyword: &str, options: &SearchOptions) -> Result<Vec<SearchResult>, String>;

    /// Series metadata plus its chapters.
    /// CONTRACT: `chapters` must be ordered oldest -> newest. Chapter numbers
    /// in the UI ("#1", selection strings like "1-10") are positions in this list.
    fn series(&self, series_id: &str) -> Result<SeriesChapters, String>;

    /// Every page image of a chapter, in reading order.
    fn pages(&self, chapter: &Chapter) -> Result<Vec<PageImage>, String>;

    /// Downloads one page image. Override when a site needs special headers
    /// or an image needs descrambling.
    fn fetch_image(&self, page: &PageImage) -> Result<Vec<u8>, String> {
        util::get_bytes(&page.url, page.referer.as_deref())
    }
}

fn loaded_sources() -> &'static [Box<dyn Source>] {
    static SOURCES: OnceLock<Vec<Box<dyn Source>>> = OnceLock::new();
    SOURCES
        .get_or_init(|| rhai_host::load_all(&rhai_host::shared_engine()))
        .as_slice()
}

/// All discovered extension sources. The first one is the default.
pub fn all() -> &'static [Box<dyn Source>] {
    loaded_sources()
}

pub fn find(id: &str) -> Option<&'static dyn Source> {
    loaded_sources()
        .iter()
        .find(|source| source.id() == id)
        .map(|source| source.as_ref())
}
