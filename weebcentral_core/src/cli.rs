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
                let cover_cache_dir = arguments.next().unwrap_or_default();
                // Optional third argument: "0" skips the per-series detail requests.
                let with_details = arguments.next().as_deref() != Some("0");
                emit_result(weebcentral::search(&keyword, &cover_cache_dir, with_details))
            }
            _ => emit_error("Search keyword is required"),
        },
        Some("chapters") => match arguments.next() {
            Some(series_id) => emit_result(weebcentral::load_chapters(&series_id)),
            None => emit_error("Series ID is required"),
        },
        Some("download") => match (
            arguments.next(),
            arguments.next(),
            arguments.next(),
            arguments.next(),
            arguments.next(),
        ) {
            (
                Some(series_id),
                Some(selection),
                Some(volume_name),
                Some(output_dir),
                Some(progress_path),
            ) => emit_result(download::download_selection(
                &series_id,
                &selection,
                &volume_name,
                &output_dir,
                &progress_path,
            )),
            _ => emit_error(
                "Usage: weebcentral_core_dev download <series-id> <selection> <volume-name> <output-dir> <progress-file>",
            ),
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
