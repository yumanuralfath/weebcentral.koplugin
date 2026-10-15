// weebcentral.com — Rhai extension for YuManga

const BASE_URL = "https://weebcentral.com";
const SEARCH_URL = "https://weebcentral.com/search/simple?location=main";

fn id() {
    "weebcentral"
}

fn name() {
    "WeebCentral"
}

fn search(keyword, cover_cache_dir, with_details) {
    let html = http_post_form(SEARCH_URL, #{ "text": keyword }, #{});
    let items = select_outer(html, "section#quick-search-result a.join-item");
    let results = [];

    for item in items {
        let hrefs = select_attr(item, "a", "href");
        if hrefs.len() == 0 {
            continue;
        }
        let href = hrefs[0];
        let series_id = id_from_href(href, "/series/");
        let titles = select_text(item, "div.flex-1");
        if series_id == "" || titles.len() == 0 || titles[0] == "" {
            continue;
        }

        let cover_url = "";
        let webps = select_attr(item, "source[type='image/webp']", "srcset");
        if webps.len() > 0 {
            cover_url = abs_url(BASE_URL, webps[0]);
        } else {
            let imgs = select_attr(item, "img", "src");
            if imgs.len() > 0 {
                cover_url = abs_url(BASE_URL, imgs[0]);
            }
        }

        let cover_path = cache_cover(series_id, cover_url, cover_cache_dir);
        let authors = "";
        let tags = "";
        let manga_type = "";
        let status = "";
        let released = "";

        if with_details != 0 {
            let page = http_get(abs_url(BASE_URL, href), #{});
            authors = list_item_value(page, "Author(s)");
            tags = list_item_value(page, "Tags(s)");
            manga_type = list_item_value(page, "Type");
            status = list_item_value(page, "Status");
            released = list_item_value(page, "Released");
        }

        results.push(#{
            "id": series_id,
            "title": titles[0],
            "url": abs_url(BASE_URL, href),
            "cover_url": cover_url,
            "cover_path": cover_path,
            "authors": authors,
            "tags": tags,
            "manga_type": manga_type,
            "status": status,
            "released": released
        });
    }

    results
}

fn series(series_id) {
    let series_url = `${BASE_URL}/series/${series_id}`;
    let page = http_get(series_url, #{});

    let titles = select_text(page, "h1");
    let title = if titles.len() > 0 { titles[0] } else { "" };

    let cover_url = "";
    let webps = select_attr(page, "picture source[type='image/webp']", "srcset");
    if webps.len() > 0 {
        cover_url = abs_url(BASE_URL, webps[0]);
    } else {
        let imgs = select_attr(page, "picture img", "src");
        if imgs.len() > 0 {
            cover_url = abs_url(BASE_URL, imgs[0]);
        }
    }

    let authors = list_item_value(page, "Author(s)");
    let tags = list_item_value(page, "Tags(s)");
    let manga_type = list_item_value(page, "Type");
    let status = list_item_value(page, "Status");
    let released = list_item_value(page, "Released");
    let description = list_item_value(page, "Description");

    let chapters_html = http_get(
        `${series_url}/full-chapter-list`,
        #{ "HX-Request": "true" }
    );

    let items = select_outer(chapters_html, "a[href^='/chapters/']");
    let chapters = [];
    for item in items {
        let hrefs = select_attr(item, "a", "href");
        if hrefs.len() == 0 {
            continue;
        }
        let href = hrefs[0];
        let chapter_id = id_from_href(href, "/chapters/");
        if chapter_id == "" {
            continue;
        }

        let chapter_titles = select_text(item, "span.grow > span:first-child");
        let chapter_title = if chapter_titles.len() > 0 {
            chapter_titles[0]
        } else {
            "Unknown chapter"
        };

        let times = select_attr(item, "time", "datetime");
        let published_at = if times.len() > 0 { times[0] } else { "" };

        chapters.push(#{
            "id": chapter_id,
            "title": chapter_title,
            "url": abs_url(BASE_URL, href),
            "published_at": published_at
        });
    }

    // Site lists newest first; UI contract is oldest -> newest.
    let reversed = [];
    let i = chapters.len() - 1;
    while i >= 0 {
        reversed.push(chapters[i]);
        i -= 1;
    }

    #{
        "id": series_id,
        "title": title,
        "url": series_url,
        "cover_url": cover_url,
        "authors": authors,
        "tags": tags,
        "manga_type": manga_type,
        "status": status,
        "released": released,
        "description": description,
        "chapters": reversed
    }
}

fn pages(chapter_id, chapter_url) {
    let html = http_get(chapter_url, #{});
    let hrefs = select_attr(html, "link[rel='preload']", "href");
    if hrefs.len() == 0 {
        throw "Could not find the first page image URL";
    }
    let first_page = abs_url(BASE_URL, hrefs[0]);

    let counts = regex_find_ints(html, "page === (\\d+) \\?");
    let page_count = 1;
    for n in counts {
        if n > page_count {
            page_count = n;
        }
    }

    let urls = derive_page_urls(first_page, page_count);
    let referer = `${BASE_URL}/chapters/${chapter_id}`;
    let result = [];
    for url in urls {
        result.push(#{ "url": url, "referer": referer });
    }
    result
}

fn list_item_value(html, label) {
    let items = select_outer(html, "li");
    for item in items {
        let labels = select_text(item, "strong");
        if labels.len() == 0 {
            continue;
        }
        if !ends_with_colon_label(labels[0], label) {
            continue;
        }

        if label == "Description" {
            let paragraphs = select_text(item, "p");
            if paragraphs.len() > 0 {
                return paragraphs[0];
            }
            return "";
        }

        let links = select_text(item, "a");
        if links.len() > 0 {
            let joined = "";
            let first = true;
            for link in links {
                if link == "" {
                    continue;
                }
                if first {
                    joined = link;
                    first = false;
                } else {
                    joined += `, ${link}`;
                }
            }
            if joined != "" {
                return joined;
            }
        }

        let spans = select_text(item, "span");
        if spans.len() > 0 {
            return spans[0];
        }
        return "";
    }
    ""
}
