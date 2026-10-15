use super::super::{parse_chapters, Html};

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
