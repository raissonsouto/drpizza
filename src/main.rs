mod addresses;
mod api;
mod config;
mod menu;
mod models;
mod order;
mod orders;
mod profile;
mod ui;
mod units;

use clap::{CommandFactory, Parser, Subcommand};
use config::AppOptions;

const VERSION_INFO: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("GIT_HASH"),
    ")\nVibecoded (kkkkkk) by Raisson Souto"
);

#[derive(Parser)]
#[command(name = "drpizza")]
#[command(about = "Aqui é RECHEIO com PIZZA!", long_about = None)]
#[command(version = VERSION_INFO)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Define a unidade antecipadamente (ID)
    #[arg(short = 'u', long = "unidade", global = true)]
    unidade: Option<u32>,
}

#[derive(Subcommand)]
enum Commands {
    /// Inicia o assistente de pedido interativo (Padrão)
    Pedir {
        /// Modo anônimo: ignora dados salvos em ~/.drpizza
        #[arg(short = 's', long = "stateless")]
        stateless: bool,

        /// Ignora cache e força busca atualizada
        #[arg(long = "no-cache")]
        no_cache: bool,
    },
    /// Exibe o cardápio completo com preços e bordas
    Menu {
        /// Exibe o cardápio sem paginação
        #[arg(long)]
        no_pagination: bool,

        /// Ignora cache e força busca atualizada
        #[arg(long = "no-cache")]
        no_cache: bool,
    },
    /// Lista unidades disponíveis (use -u ID para detalhes)
    Unidades {
        /// Mostra todas as unidades (sem filtro por bairro)
        #[arg(short = 'a', long = "all")]
        all: bool,

        /// Exibe visão detalhada das unidades
        #[arg(long = "detalhes")]
        detalhes: bool,

        /// Define uma unidade como padrão para o endereço atual
        #[arg(short = 'd', long = "default")]
        default: Option<u32>,

        /// Remove a unidade padrão do endereço atual
        #[arg(long = "no-default")]
        no_default: bool,
    },
    /// Mostra status do último pedido
    Pedido,
    /// Lista histórico de pedidos
    Pedidos,
    /// Gerencia o perfil local (nome, telefone)
    Perfil {
        /// Editar perfil interativamente
        #[arg(short = 'e', long)]
        edit: bool,
    },
    /// Gerencia endereços de entrega
    #[command(name = "enderecos")]
    Enderecos {
        /// Exibe endereços e sai (sem menu interativo)
        #[arg(short = 's', long = "show")]
        show: bool,

        /// Define endereço padrão pelo índice (1-based)
        #[arg(short = 'd', long = "default")]
        default: Option<usize>,

        /// Remove endereço pelo índice (1-based)
        #[arg(short = 'r', long = "remove")]
        remove: Option<usize>,

        /// Adiciona novo endereço diretamente
        #[arg(short = 'a', long = "add")]
        add: bool,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let base_opts = |stateless: bool, no_cache: bool| AppOptions {
        stateless,
        no_cache,
        unit_id: cli.unidade,
    };

    match cli.command {
        Some(Commands::Pedir {
            stateless,
            no_cache,
        }) => {
            order::start_order_flow(&base_opts(stateless, no_cache)).await;
        }
        Some(Commands::Menu {
            no_pagination,
            no_cache,
        }) => {
            menu::list_menu(&base_opts(false, no_cache), no_pagination).await;
        }
        Some(Commands::Unidades {
            all,
            detalhes,
            default,
            no_default,
        }) => {
            units::list_units(&base_opts(false, false), all, detalhes, default, no_default).await;
        }
        Some(Commands::Pedido) => {
            orders::show_last_order(&base_opts(false, false)).await;
        }
        Some(Commands::Pedidos) => {
            orders::show_order_history(&base_opts(false, false)).await;
        }
        Some(Commands::Perfil { edit }) => {
            profile::show_profile(&base_opts(false, false), edit).await;
        }
        Some(Commands::Enderecos {
            show,
            default,
            remove,
            add,
        }) => {
            addresses::manage_addresses(&base_opts(false, false), show, default, remove, add).await;
        }
        None => {
            Cli::command().print_help().unwrap();
        }
    }
}
