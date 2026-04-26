use clap::{CommandFactory, Parser, Subcommand};

use crate::addresses;
use crate::config::AppOptions;
use crate::menu;
use crate::order;
use crate::orders;
use crate::points;
use crate::profile;
use crate::units;

const VERSION_INFO: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("GIT_HASH"),
    ")\nby Raisson Souto"
);

#[derive(Parser)]
#[command(name = "drpizza")]
#[command(about = "Aqui é RECHEIO com PIZZA!", long_about = None)]
#[command(version = VERSION_INFO)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Define a unidade antecipadamente (ID da lista)
    #[arg(short = 'u', long = "unidade", global = true)]
    unidade: Option<usize>,
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
    #[command(name = "cardapio")]
    Cardapio {
        /// Exibe o cardápio sem paginação
        #[arg(long)]
        no_pagination: bool,

        /// Ignora cache e força busca atualizada
        #[arg(long = "no-cache")]
        no_cache: bool,
    },
    /// Lista unidades disponíveis (use -u ID da lista para detalhes)
    Unidades {
        /// Mostra todas as unidades (sem filtro por bairro)
        #[arg(short = 'a', long = "all")]
        all: bool,

        /// Exibe visão detalhada das unidades
        #[arg(long = "detalhes")]
        detalhes: bool,

        /// Define uma unidade como padrão para o endereço atual (ID da lista)
        #[arg(short = 'd', long = "default")]
        default: Option<usize>,

        /// Remove a unidade padrão do endereço atual
        #[arg(long = "no-default")]
        no_default: bool,
    },
    /// Mostra status do último pedido
    #[command(name = "status", alias = "pedido")]
    Status,
    /// Lista histórico de pedidos
    Pedidos,
    /// Mostra pontos acumulados e benefícios de fidelidade
    Pontos,
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

pub async fn run() {
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
        Some(Commands::Cardapio {
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
        Some(Commands::Status) => {
            orders::show_last_order(&base_opts(false, false)).await;
        }
        Some(Commands::Pedidos) => {
            orders::show_order_history(&base_opts(false, false)).await;
        }
        Some(Commands::Pontos) => {
            points::show_points(&base_opts(false, false)).await;
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
