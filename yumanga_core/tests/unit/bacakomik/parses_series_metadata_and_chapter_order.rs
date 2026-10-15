use super::super::{parse_series, Html};

#[test]
fn parses_series_metadata_and_returns_oldest_chapter_first() {
    let html = r#"<h1 class="entry-title">Komik <span itemprop="name">One Piece</span></h1>
        <div class="shortcsc">A pirate adventure.</div>
        <div class="thumb"><img data-lazy-src="https://cdn.example/cover.png"></div>
        <div class="infox"><div class="spe">
          <span><b>Status:</b> Berjalan</span>
          <span><b>Jenis Komik:</b><a>Manga</a></span>
          <span><b>Author:</b><a>ODA Eiichiro</a></span>
          <span><b>Artis:</b><a>ODA Eiichiro</a></span>
          <span><b>Rilis:</b><a>1997</a></span>
        </div><div class="genre-info"><a>Action</a><a>Adventure</a></div></div>
        <div id="chapter_list"><ul>
          <li><span class="lchx"><a href="https://bacakomik.my/one-piece-chapter-2/"><chapter>2</chapter></a></span><span class="dt">2 hari</span></li>
          <li><span class="lchx"><a href="https://bacakomik.my/one-piece-chapter-1/"><chapter>1</chapter></a></span><span class="dt">3 hari</span></li>
        </ul></div>"#;
    let series = parse_series(
        "one-piece",
        "https://bacakomik.my/komik/one-piece/",
        &Html::parse_document(html),
    )
    .unwrap();
    assert_eq!(series.title, "One Piece");
    assert_eq!(series.description, "A pirate adventure.");
    assert_eq!(series.cover_url, "https://cdn.example/cover.png");
    assert_eq!(series.authors, "ODA Eiichiro");
    assert_eq!(series.tags, "Action, Adventure");
    assert_eq!(series.manga_type, "Manga");
    assert_eq!(series.status, "Berjalan");
    assert_eq!(series.released, "1997");
    assert_eq!(series.chapters.len(), 2);
    assert_eq!(series.chapters[0].id, "one-piece-chapter-1");
    assert_eq!(series.chapters[0].title, "Chapter 1");
    assert_eq!(series.chapters[1].id, "one-piece-chapter-2");
}
