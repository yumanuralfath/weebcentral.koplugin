mod cli;
mod download;
mod models;
mod util;
mod weebcentral;

fn main() -> std::process::ExitCode {
    cli::run()
}
