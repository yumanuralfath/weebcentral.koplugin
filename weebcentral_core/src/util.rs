use std::io::Read;

pub const BASE_URL: &str = "https://weebcentral.com";
pub const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36";

pub fn get_text(url: &str) -> Result<String, String> {
    let response = ureq::get(url)
        .set("User-Agent", USER_AGENT)
        .call()
        .map_err(|error| error.to_string())?;
    read_response_text(response)
}

pub fn read_response_text(response: ureq::Response) -> Result<String, String> {
    let mut text = String::new();
    response
        .into_reader()
        .read_to_string(&mut text)
        .map_err(|error| error.to_string())?;
    Ok(text)
}

pub fn id_from_href<'a>(href: &'a str, route: &str) -> &'a str {
    href.split_once(route)
        .map(|(_, remainder)| remainder)
        .unwrap_or_default()
        .split(['/', '?'])
        .next()
        .unwrap_or_default()
}

pub fn normalize_whitespace(value: String) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn absolute_url(value: &str) -> String {
    let url = value.split_whitespace().next().unwrap_or_default();
    if url.starts_with("http://") || url.starts_with("https://") {
        url.to_string()
    } else if url.starts_with("//") {
        format!("https:{url}")
    } else if url.starts_with('/') {
        format!("{BASE_URL}{url}")
    } else if url.is_empty() {
        String::new()
    } else {
        format!("{BASE_URL}/{url}")
    }
}

pub fn validate_id(value: &str) -> Result<(), String> {
    if value.is_empty()
        || !value.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '-' || character == '_'
        })
    {
        return Err("Invalid series or chapter ID".to_string());
    }
    Ok(())
}

pub fn safe_filename(value: &str) -> String {
    let filename: String = value
        .chars()
        .map(|character| {
            if character.is_control()
                || matches!(
                    character,
                    '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'
                )
            {
                '_'
            } else {
                character
            }
        })
        .collect();
    let filename = filename
        .trim()
        .trim_matches(|character| character == '.' || character == ' ');
    if filename.is_empty() {
        "chapter".to_string()
    } else {
        filename.to_string()
    }
}
