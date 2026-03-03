mod addresses;
mod api;
mod cli;
mod config;
mod menu;
mod models;
mod order;
mod orders;
mod profile;
mod ui;
mod units;

#[tokio::main]
async fn main() {
    cli::run().await;
}
