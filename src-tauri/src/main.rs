// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod singleton;
mod utils;

#[tokio::main]
async fn main() {
    zerolaunch_rs_lib::run()
}
