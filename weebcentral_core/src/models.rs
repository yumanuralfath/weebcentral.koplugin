use serde::Serialize;

#[derive(Serialize)]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub url: String,
    pub cover_url: String,
    pub cover_path: String,
}

#[derive(Serialize, Debug)]
pub struct Chapter {
    pub id: String,
    pub title: String,
    pub url: String,
    pub published_at: String,
}

#[derive(Serialize, Debug)]
pub struct SeriesChapters {
    pub id: String,
    pub title: String,
    pub url: String,
    pub cover_url: String,
    pub authors: String,
    pub tags: String,
    pub manga_type: String,
    pub status: String,
    pub description: String,
    pub chapters: Vec<Chapter>,
}

#[derive(Serialize)]
pub struct DownloadResult {
    pub paths: Vec<String>,
    pub skipped: usize,
}
