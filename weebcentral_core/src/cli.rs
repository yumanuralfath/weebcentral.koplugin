use serde::Serialize;
use serde_json::json;
use std::env;
use std::process::ExitCode;

use crate::{download, weebcentral};

pub fn run() -> ExitCode {
    let mut arguments = env::args().skip(1);
    match arguments.next().as_deref() {
        Some("search") => match arguments.next() {
            Some(keyword) if !keyword.trim().is_empty() => {
                emit_result(weebcentral::search(&keyword))
            }
            _ => emit_error("Search keyword is required"),
        },
        Some("chapters") => match arguments.next() {
            Some(series_id) => emit_result(weebcentral::load_chapters(&series_id)),
            None => emit_error("Series ID is required"),
        },
        Some("download") => match (arguments.next(), arguments.next(), arguments.next()) {
            (Some(chapter_id), Some(title), Some(output_dir)) => {
                emit_result(download::download_chapter(&chapter_id, &title, &output_dir))
            }
            _ => {
                emit_error("Usage: weebcentral_core_dev download <chapter-id> <title> <output-dir>")
            }
        },
        _ => emit_error("Usage: weebcentral_core_dev search <keyword>"),
    }
}

fn emit_result<T: Serialize>(result: Result<T, String>) -> ExitCode {
    match result {
        Ok(data) => {
            println!("{}", json!({ "ok": true, "data": data }));
            ExitCode::SUCCESS
        }
        Err(error) => emit_error(error),
    }
}

fn emit_error(error: impl std::fmt::Display) -> ExitCode {
    println!("{}", json!({ "ok": false, "error": error.to_string() }));
    ExitCode::FAILURE
}
