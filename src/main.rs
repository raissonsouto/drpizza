mod models;
mod data;
mod services;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "drpizza")]
#[command(about = "Aqui é RECHEIO com PIZZA!", long_about = None)]
#[command(version = "1.0")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Define a unidade antecipadamente (ID)
    #[arg(short, long)]
    unit: Option<u32>,
}

#[derive(Subcommand)]
enum Commands {
    /// Inicia o assistente de pedido interativo (Padrão)
    Order,
    /// Exibe o cardápio completo com preços e bordas
    Menu,
    /// Lista todas as unidades disponíveis e endereços
    Units,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Menu) => {
            services::list_menu();
        }
        Some(Commands::Units) => {
            services::list_units();
        }
        Some(Commands::Order) => {
            // Pass the optional unit flag if present
            services::start_order_flow(cli.unit);
        }
        None => {
            // Default behavior if no command is given
            services::start_order_flow(cli.unit);
        }
    }
}