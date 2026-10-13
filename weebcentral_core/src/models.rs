use serde::Serialize;

#[derive(Serialize)]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub url: String,
    pub cover_url: String,
}

#[derive(Serialize)]
pub struct Chapter {
    pub id: String,
    pub title: String,
    pub url: String,
    pub published_at: String,
}

#[derive(Serialize)]
pub struct SeriesChapters {
    pub title: String,
    pub chapters: Vec<Chapter>,
}

#[derive(Serialize)]
pub struct DownloadResult {
    pub path: String,
    pub already_exists: bool,
}
