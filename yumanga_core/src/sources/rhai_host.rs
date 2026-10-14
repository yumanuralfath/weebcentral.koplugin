//! Rhai host: shared engine, native helpers, and [`RhaiSource`].
//!
//! Scripts live under `YuManga.koplugin/extensions/*.rhai` and export
//! `id`, `name`, `search`, `series`, and `pages`.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rhai::{AST, Array, Dynamic, Engine, Map, Scope};
use scraper::{Html, Selector};

use super::{SearchOptions, Source};
use crate::models::{Chapter, PageImage, SearchResult, SeriesChapters};
use crate::util::{
    USER_AGENT, absolute_url, id_from_href, normalize_whitespace, read_response_text, validate_id,
};

/// One compiled extension script.
pub struct RhaiSource {
    id: String,
    name: String,
    engine: Arc<Engine>,
    ast: AST,
    /// Top-level `const` / `let` from the script (`call_fn` alone does not run them).
    globals: Scope<'static>,
}

impl Source for RhaiSource {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn search(&self, keyword: &str, options: &SearchOptions) -> Result<Vec<SearchResult>, String> {
        let with_details: i32 = if options.with_details { 1 } else { 0 };
        let value = self.call(
            "search",
            (
                keyword.to_string(),
                options.cover_cache_dir.to_string(),
                with_details,
            ),
        )?;
        let array = value
            .into_array()
            .map_err(|_| "search() must return an array".to_string())?;
        array.iter().map(dynamic_to_search_result).collect()
    }

    fn series(&self, series_id: &str) -> Result<SeriesChapters, String> {
        validate_id(series_id)?;
        let value = self.call("series", (series_id.to_string(),))?;
        dynamic_to_series(&value)
    }

    fn pages(&self, chapter: &Chapter) -> Result<Vec<PageImage>, String> {
        let value = self.call("pages", (chapter.id.clone(), chapter.url.clone()))?;
        let array = value
            .into_array()
            .map_err(|_| "pages() must return an array".to_string())?;
        array.iter().map(dynamic_to_page).collect()
    }
}

impl RhaiSource {
    fn call<A: rhai::FuncArgs>(&self, name: &str, args: A) -> Result<Dynamic, String> {
        let mut scope = self.globals.clone();
        self.engine
            .call_fn(&mut scope, &self.ast, name, args)
            .map_err(|error| error.to_string())
    }
}

/// Build the shared engine with host helpers registered once.
pub fn shared_engine() -> Arc<Engine> {
    let mut engine = Engine::new();
    engine.set_max_expr_depths(64, 64);
    engine.set_max_operations(500_000);

    engine.register_fn("http_get", host_http_get);
    engine.register_fn("http_post_form", host_http_post_form);
    engine.register_fn("select_outer", host_select_outer);
    engine.register_fn("select_attr", host_select_attr);
    engine.register_fn("select_text", host_select_text);
    engine.register_fn("abs_url", host_abs_url);
    engine.register_fn("id_from_href", host_id_from_href);
    engine.register_fn("normalize_ws", host_normalize_ws);
    engine.register_fn("regex_captures", host_regex_captures);
    engine.register_fn("regex_find_ints", host_regex_find_ints);
    engine.register_fn("cache_cover", host_cache_cover);
    engine.register_fn("derive_page_urls", host_derive_page_urls);
    engine.register_fn("ends_with_colon_label", host_ends_with_colon_label);

    Arc::new(engine)
}

/// Load every `*.rhai` under the known extension directories.
pub fn load_all(engine: &Arc<Engine>) -> Vec<Box<dyn Source>> {
    let mut sources = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for dir in extension_dirs() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        let mut paths: Vec<PathBuf> = entries
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension().and_then(|extension| extension.to_str()) == Some("rhai")
            })
            .collect();
        paths.sort();

        for path in paths {
            match load_script(engine, &path) {
                Ok(source) => {
                    if seen.insert(source.id.clone()) {
                        sources.push(Box::new(source) as Box<dyn Source>);
                    }
                }
                Err(error) => {
                    eprintln!("yumanga: skip {}: {error}", path.display());
                }
            }
        }
    }
    sources
}

fn extension_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        // .../YuManga.koplugin/bin/yumanga_core_dev → .../YuManga.koplugin/extensions
        if let Some(plugin_root) = exe.parent().and_then(Path::parent) {
            dirs.push(plugin_root.join("extensions"));
        }
    }
    // Native / test builds from the repo.
    dirs.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../YuManga.koplugin/extensions"));
    dirs
}

fn load_script(engine: &Arc<Engine>, path: &Path) -> Result<RhaiSource, String> {
    let source = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let ast = engine
        .compile(&source)
        .map_err(|error| format!("compile: {error}"))?;

    // Execute top-level statements so `const BASE_URL` etc. land in the scope.
    // `call_fn` alone only runs the named function body.
    let mut globals = Scope::new();
    engine
        .run_ast_with_scope(&mut globals, &ast)
        .map_err(|error| format!("init: {error}"))?;

    let id: String = engine
        .call_fn(&mut globals.clone(), &ast, "id", ())
        .map_err(|error| format!("id(): {error}"))?;
    let name: String = engine
        .call_fn(&mut globals.clone(), &ast, "name", ())
        .map_err(|error| format!("name(): {error}"))?;
    if id.is_empty() || name.is_empty() {
        return Err("id() and name() must be non-empty".to_string());
    }
    Ok(RhaiSource {
        id,
        name,
        engine: Arc::clone(engine),
        ast,
        globals,
    })
}

// --- Host helpers ------------------------------------------------------------

fn map_to_header_pairs(headers: &Map) -> Vec<(String, String)> {
    headers
        .iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect()
}

fn host_http_get(url: &str, headers: Map) -> Result<String, Box<rhai::EvalAltResult>> {
    if headers.is_empty() {
        return crate::util::get_text(url).map_err(|error| error.into());
    }
    let mut request = ureq::get(url).set("User-Agent", USER_AGENT);
    for (key, value) in map_to_header_pairs(&headers) {
        request = request.set(&key, &value);
    }
    let response = request.call().map_err(|error| error.to_string())?;
    read_response_text(response).map_err(|error| error.into())
}

fn host_http_post_form(
    url: &str,
    form: Map,
    headers: Map,
) -> Result<String, Box<rhai::EvalAltResult>> {
    let pairs: Vec<(String, String)> = form
        .iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect();
    let form_refs: Vec<(&str, &str)> = pairs
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();

    let mut request = ureq::post(url).set("User-Agent", USER_AGENT);
    for (key, value) in map_to_header_pairs(&headers) {
        request = request.set(&key, &value);
    }
    let response = request
        .send_form(&form_refs)
        .map_err(|error| error.to_string())?;
    read_response_text(response).map_err(|error| error.into())
}

fn parse_html(html: &str) -> Html {
    // Fragments from select_outer must round-trip; full pages use the document parser.
    if html.contains("<html") || html.contains("<HTML") || html.contains("<!DOCTYPE") {
        Html::parse_document(html)
    } else {
        Html::parse_fragment(html)
    }
}

fn host_select_outer(html: &str, css: &str) -> Result<Array, Box<rhai::EvalAltResult>> {
    let selector = Selector::parse(css).map_err(|error| format!("bad selector: {error:?}"))?;
    let document = parse_html(html);
    Ok(document
        .select(&selector)
        .map(|element| Dynamic::from(element.html()))
        .collect())
}

fn host_select_attr(html: &str, css: &str, attr: &str) -> Result<Array, Box<rhai::EvalAltResult>> {
    let selector = Selector::parse(css).map_err(|error| format!("bad selector: {error:?}"))?;
    let document = parse_html(html);
    Ok(document
        .select(&selector)
        .filter_map(|element| {
            element
                .value()
                .attr(attr)
                .map(|value| Dynamic::from(value.to_string()))
        })
        .collect())
}

fn host_select_text(html: &str, css: &str) -> Result<Array, Box<rhai::EvalAltResult>> {
    let selector = Selector::parse(css).map_err(|error| format!("bad selector: {error:?}"))?;
    let document = parse_html(html);
    Ok(document
        .select(&selector)
        .map(|element| {
            Dynamic::from(normalize_whitespace(
                element.text().collect::<Vec<_>>().join(" "),
            ))
        })
        .collect())
}

fn host_abs_url(base: &str, href: &str) -> String {
    absolute_url(base, href)
}

fn host_id_from_href(href: &str, route: &str) -> String {
    id_from_href(href, route).to_string()
}

fn host_normalize_ws(value: &str) -> String {
    normalize_whitespace(value.to_string())
}

fn host_regex_captures(text: &str, pattern: &str) -> Result<Array, Box<rhai::EvalAltResult>> {
    let regex = regex::Regex::new(pattern).map_err(|error| error.to_string())?;
    let Some(captures) = regex.captures(text) else {
        return Ok(Array::new());
    };
    Ok(captures
        .iter()
        .skip(1)
        .map(|group| {
            Dynamic::from(
                group
                    .map(|matched| matched.as_str().to_string())
                    .unwrap_or_default(),
            )
        })
        .collect())
}

fn host_regex_find_ints(text: &str, pattern: &str) -> Result<Array, Box<rhai::EvalAltResult>> {
    let regex = regex::Regex::new(pattern).map_err(|error| error.to_string())?;
    Ok(regex
        .captures_iter(text)
        .filter_map(|capture| capture.get(1))
        .filter_map(|matched| matched.as_str().parse::<i32>().ok())
        .map(Dynamic::from)
        .collect())
}

fn host_cache_cover(id: &str, cover_url: &str, cache_dir: &str) -> String {
    cache_cover(id, cover_url, Path::new(cache_dir))
}

fn host_derive_page_urls(first_page: &str, count: i32) -> Result<Array, Box<rhai::EvalAltResult>> {
    let urls = derive_page_urls(first_page, count.max(1) as usize)?;
    Ok(urls.into_iter().map(Dynamic::from).collect())
}

/// True when `text` equals `label`, ignoring a trailing colon on either side.
fn host_ends_with_colon_label(text: &str, label: &str) -> bool {
    text.trim().trim_end_matches(':') == label.trim().trim_end_matches(':')
}

fn cache_cover(id: &str, cover_url: &str, cache_dir: &Path) -> String {
    if cover_url.is_empty() || cache_dir.as_os_str().is_empty() {
        return String::new();
    }
    let _ = fs::create_dir_all(cache_dir);
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
    let _ = output.flush();
    path.to_string_lossy().into_owned()
}

/// Expand a first-page CDN URL (`...-001.png`) into every page URL.
pub fn derive_page_urls(first_page: &str, count: usize) -> Result<Vec<String>, String> {
    let pattern =
        regex::Regex::new(r"^(?P<prefix>.+-)(?P<number>\d+)(?P<extension>\.\w+)(?:\?.*)?$")
            .map_err(|error| error.to_string())?;
    let captures = pattern
        .captures(first_page)
        .ok_or_else(|| format!("Unrecognized page image URL: {first_page}"))?;
    let prefix = &captures["prefix"];
    let width = captures["number"].len();
    let extension = &captures["extension"];
    Ok((1..=count)
        .map(|number| format!("{prefix}{number:0width$}{extension}"))
        .collect())
}

// --- Dynamic → Rust ----------------------------------------------------------

fn map_str(map: &Map, key: &str) -> String {
    map.get(key)
        .map(|value| value.to_string())
        .unwrap_or_default()
}

fn dynamic_as_map(value: &Dynamic) -> Result<Map, String> {
    value
        .clone()
        .try_cast::<Map>()
        .ok_or_else(|| "expected a map".to_string())
}

fn dynamic_to_search_result(value: &Dynamic) -> Result<SearchResult, String> {
    let map = dynamic_as_map(value)?;
    Ok(SearchResult {
        id: map_str(&map, "id"),
        title: map_str(&map, "title"),
        url: map_str(&map, "url"),
        cover_url: map_str(&map, "cover_url"),
        cover_path: map_str(&map, "cover_path"),
        authors: map_str(&map, "authors"),
        tags: map_str(&map, "tags"),
        manga_type: map_str(&map, "manga_type"),
        status: map_str(&map, "status"),
        released: map_str(&map, "released"),
    })
}

fn dynamic_to_series(value: &Dynamic) -> Result<SeriesChapters, String> {
    let map = dynamic_as_map(value)?;
    let chapters_value = map
        .get("chapters")
        .cloned()
        .unwrap_or_else(|| Dynamic::from(Array::new()));
    let chapters_array = chapters_value
        .into_array()
        .map_err(|_| "series().chapters must be an array".to_string())?;
    let chapters = chapters_array
        .iter()
        .map(dynamic_to_chapter)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SeriesChapters {
        id: map_str(&map, "id"),
        title: map_str(&map, "title"),
        url: map_str(&map, "url"),
        cover_url: map_str(&map, "cover_url"),
        authors: map_str(&map, "authors"),
        tags: map_str(&map, "tags"),
        manga_type: map_str(&map, "manga_type"),
        status: map_str(&map, "status"),
        released: map_str(&map, "released"),
        description: map_str(&map, "description"),
        chapters,
    })
}

fn dynamic_to_chapter(value: &Dynamic) -> Result<Chapter, String> {
    let map = dynamic_as_map(value)?;
    Ok(Chapter {
        id: map_str(&map, "id"),
        title: map_str(&map, "title"),
        url: map_str(&map, "url"),
        published_at: map_str(&map, "published_at"),
    })
}

fn dynamic_to_page(value: &Dynamic) -> Result<PageImage, String> {
    let map = dynamic_as_map(value)?;
    let referer = map_str(&map, "referer");
    Ok(PageImage {
        url: map_str(&map, "url"),
        referer: if referer.is_empty() {
            None
        } else {
            Some(referer)
        },
    })
}

#[cfg(test)]
mod tests {
    use super::{
        derive_page_urls, host_ends_with_colon_label, host_select_text, load_all, shared_engine,
    };

    #[test]
    fn derives_every_page_url_from_the_first_one() {
        let urls = derive_page_urls("https://cdn.example/m/001-009.png", 3).unwrap();
        assert_eq!(
            urls,
            [
                "https://cdn.example/m/001-001.png",
                "https://cdn.example/m/001-002.png",
                "https://cdn.example/m/001-003.png"
            ]
        );
    }

    #[test]
    fn unrecognized_page_url_is_an_error() {
        assert!(derive_page_urls("https://cdn.example/nope", 1).is_err());
    }

    #[test]
    fn loads_bundled_weebcentral_extension() {
        let sources = load_all(&shared_engine());
        assert!(
            sources.iter().any(|source| source.id() == "weebcentral"),
            "expected weebcentral.rhai under YuManga.koplugin/extensions"
        );
    }

    #[test]
    fn top_level_consts_are_visible_to_script_functions() {
        use rhai::{Engine, Scope};

        let engine = Engine::new();
        let ast = engine
            .compile(
                r#"
                const SEARCH_URL = "https://example.com/search";
                fn get_url() { SEARCH_URL }
                "#,
            )
            .unwrap();
        let mut globals = Scope::new();
        engine.run_ast_with_scope(&mut globals, &ast).unwrap();
        let url: String = engine
            .call_fn(&mut globals.clone(), &ast, "get_url", ())
            .unwrap();
        assert_eq!(url, "https://example.com/search");
    }

    #[test]
    fn label_match_ignores_trailing_colon() {
        assert!(host_ends_with_colon_label("Author(s):", "Author(s)"));
        assert!(host_ends_with_colon_label("Type", "Type:"));
        assert!(!host_ends_with_colon_label("Tags", "Type"));
    }

    #[test]
    fn select_text_reads_fragment() {
        let texts =
            host_select_text(r#"<li><strong>Type: </strong><a>Manga</a></li>"#, "strong").unwrap();
        assert_eq!(texts.len(), 1);
        assert_eq!(texts[0].to_string(), "Type:");
    }
}
