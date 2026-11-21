mod addresses;
mod api;
mod cli;
mod config;
mod menu;
mod models;
mod notify;
mod order;
mod orders;
mod points;
mod profile;
mod ui;
mod units;

#[tokio::main]
async fn main() {
    cli::run().await;
}
