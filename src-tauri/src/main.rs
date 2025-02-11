// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use project_dave_lib::logging;

#[tokio::main]
async fn main() {
    let _ = fix_path_env::fix();
    logging::setup_logging();
    project_dave_lib::run().await;
}
