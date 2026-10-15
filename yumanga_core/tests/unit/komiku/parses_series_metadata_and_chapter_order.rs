use super::super::{parse_series, Html};

#[test]
fn parses_series_metadata_and_returns_oldest_chapter_first() {
    let html = r#"<h1><span>Komik <span itemprop="name">One Piece</span></span></h1>
        <section id="Informasi"><div class="ims"><img src="https://thumbnail.komiku.to/one-piece.jpg"></div></section>
        <p class="desc">Pirate adventures.</p>
        <table class="inftable"><tr><td>Tipe:</td><td>Manga</td></tr><tr><td>Genre:</td><td><ul><li><a>Adventure</a></li><li><a>Fantasy</a></li></ul></td></tr><tr><td>Author:</td><td>Eiichiro Oda</td></tr><tr><td>Status:</td><td>Ongoing</td></tr></table>
        <table><tbody id="daftarChapter">
            <tr itemprop="itemListElement"><td class="judulseries"><a href="/one-piece-chapter-2/" itemprop="url"><span itemprop="name"><b>Chapter 2</b></span></a></td><td class="tanggalseries">02/01/2026</td></tr>
            <tr itemprop="itemListElement"><td class="judulseries"><a href="/one-piece-chapter-1/" itemprop="url"><span itemprop="name"><b>Chapter 1</b></span></a></td><td class="tanggalseries">01/01/2026</td></tr>
        </tbody></table>"#;
    let series = parse_series(
        "komik-one-piece-indo",
        "https://komiku.org/manga/komik-one-piece-indo/",
        &Html::parse_document(html),
    )
    .unwrap();
    assert_eq!(series.title, "One Piece");
    assert_eq!(
        series.cover_url,
        "https://thumbnail.komiku.to/one-piece.jpg"
    );
    assert_eq!(series.description, "Pirate adventures.");
    assert_eq!(series.authors, "Eiichiro Oda");
    assert_eq!(series.tags, "Adventure, Fantasy");
    assert_eq!(series.manga_type, "Manga");
    assert_eq!(series.status, "Ongoing");
    assert_eq!(series.chapters.len(), 2);
    assert_eq!(series.chapters[0].id, "one-piece-chapter-1");
    assert_eq!(series.chapters[0].title, "Chapter 1");
    assert_eq!(series.chapters[1].id, "one-piece-chapter-2");
}
