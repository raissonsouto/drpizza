use crate::api;
use crate::config::{self, AppOptions};
use crate::menu;
use crate::models::{CartItem, SavedAddress, UserConfig};
use crate::ui;
use crate::units;
use colored::*;

pub async fn start_order_flow(opts: &AppOptions) {
    let (selected_unit, ctx) = units::select_unit_and_context(opts).await;
    let menu_data = config::get_menu_data(&ctx, opts).await;

    if menu_data.is_empty() {
        println!("{}", "O cardápio está vazio. Verifique a conexão.".red());
        return;
    }

    println!("{} {}", "Conectado:".green(), selected_unit.name);

    let mut cart: Vec<CartItem> = Vec::new();
    let mut total_price = 0.0;
    let mut ordering = true;

    while ordering {
        match menu::browse_menu_select(&menu_data) {
            Some(sel) => {
                let mut item_price = sel.item.get_current_price();
                let flavor_names: Vec<String> =
                    sel.flavors.iter().map(|f| f.name.clone()).collect();
                for f in &sel.flavors {
                    item_price += f.price;
                }
                let crust_name = sel
                    .crust
                    .as_ref()
                    .map(|c| c.name.clone())
                    .unwrap_or_else(|| "Tradicional".to_string());
                if let Some(c) = &sel.crust {
                    item_price += c.price;
                }

                cart.push(CartItem {
                    name: sel.item.name.clone(),
                    flavors: flavor_names.clone(),
                    crust: crust_name.clone(),
                    price: item_price,
                });
                total_price += item_price;

                let mut desc = sel.item.name.green().to_string();
                if !flavor_names.is_empty() {
                    desc.push_str(&format!(" ({})", flavor_names.join(", ")));
                }
                println!(
                    "{} adicionado! 🛒 Subtotal: {}",
                    desc,
                    format!("R$ {:.2}", total_price).green().bold()
                );

                let cont = ui::read_input("Pedir mais algo? (S/N): ");
                if cont.to_uppercase() != "S" {
                    ordering = false;
                }
            }
            None => {
                ordering = false;
            }
        }
    }

    if cart.is_empty() {
        println!("Carrinho vazio.");
        return;
    }

    // Checkout
    println!("\n{}", "📝 RESUMO DO PEDIDO".on_white().black().bold());
    println!("{}", "-------------------".bright_black());

    for item in &cart {
        println!(
            "• {:.<40} {}",
            item.name,
            format!("R$ {:.2}", item.price).green()
        );
        if !item.flavors.is_empty() {
            println!(
                "  Sabores: {}",
                item.flavors.join(", ").truecolor(150, 150, 150)
            );
        }
        println!("  Borda: {}", item.crust.truecolor(150, 150, 150));
    }
    println!("{}", "-------------------".bright_black());
    println!(
        "💰 SUBTOTAL: {}",
        format!("R$ {:.2}", total_price).green().bold()
    );

    // Gather customer data if incomplete
    let user_config = config::load_user_config(opts);
    let (customer_name, customer_phone) = gather_customer_info(&user_config);

    println!(
        "{} {}  {} {}",
        "Cliente:".yellow(),
        customer_name.bold(),
        "Telefone:".yellow(),
        customer_phone
    );

    // Register client to get client_id
    ensure_client_id(&ctx, &customer_name, &customer_phone, &user_config, opts).await;

    // Address selection
    let (street, number, _complement, neighborhood, city, state, zip_code, _landmark) =
        select_delivery_address(&user_config, opts).await;

    // Calculate delivery tax
    let sp = ui::Spinner::new("Calculando frete...");
    let delivery_fee = match api::calculate_delivery_tax(
        &ctx,
        &street,
        &number,
        &neighborhood,
        &city,
        &state,
        &zip_code,
    )
    .await
    {
        Ok(fee) => {
            sp.stop();
            println!("🚚 Taxa de entrega: {}", format!("R$ {:.2}", fee).yellow());
            fee
        }
        Err(e) => {
            drop(sp);
            eprintln!("Erro ao calcular taxa de entrega: {}", e);
            0.0
        }
    };

    let grand_total = total_price + delivery_fee;
    println!(
        "💰 TOTAL: {}",
        format!("R$ {:.2}", grand_total).green().bold().on_black()
    );

    // Loyalty
    let points_earned = total_price as u32;
    println!(
        "{}",
        format!("🎁 Fidelidade: +{} pontos", points_earned).magenta()
    );

    let ver_premios = ui::read_input("Ver catálogo de prêmios? (S/N): ");
    if ver_premios.to_uppercase() == "S" {
        list_rewards();
    }

    ui::read_input("Pressione ENTER para confirmar o pedido...");
    println!(
        "{}",
        "🚀 Enviando pedido... Aguarde a pizza! 🍕".green().bold()
    );
}

fn gather_customer_info(user_config: &Option<UserConfig>) -> (String, String) {
    let mut name = String::new();
    let mut phone = String::new();

    if let Some(cfg) = user_config {
        name = cfg.name.clone();
        phone = cfg.phone.clone();
    }

    if name.is_empty() {
        name = ui::read_input("Seu nome: ");
    }

    if phone.is_empty() {
        phone = ui::read_input("Seu telefone: ");
    }

    (name, phone)
}

async fn select_delivery_address(
    user_config: &Option<UserConfig>,
    opts: &AppOptions,
) -> (
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
) {
    if let Some(config) = user_config {
        if !config.addresses.is_empty() {
            println!("\n{}", "📍 Endereço de entrega:".yellow().bold());
            for (i, addr) in config.addresses.iter().enumerate() {
                let default_marker = if config.endereco_padrao == Some(i) {
                    " ★"
                } else {
                    ""
                };
                println!(
                    "  [{}] {}{} - {}, {} - {}, {}",
                    i + 1,
                    addr.label.bold(),
                    default_marker.yellow(),
                    addr.street,
                    addr.number,
                    addr.neighborhood,
                    addr.city
                );
            }
            println!("  [{}] Novo endereço", config.addresses.len() + 1);

            let choice = ui::read_input("Escolha: ");
            if let Ok(idx) = choice.parse::<usize>() {
                if idx >= 1 && idx <= config.addresses.len() {
                    let addr = &config.addresses[idx - 1];
                    return (
                        addr.street.clone(),
                        addr.number.clone(),
                        addr.complement.clone(),
                        addr.neighborhood.clone(),
                        addr.city.clone(),
                        addr.state.clone(),
                        addr.cep.replace('-', ""),
                        addr.landmark.clone(),
                    );
                }
            }
        }
    }

    // New address via CEP lookup
    collect_new_address(opts).await
}

async fn collect_new_address(
    opts: &AppOptions,
) -> (
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
) {
    let cep = ui::read_input("Digite o CEP: ");

    let sp = ui::Spinner::new("Buscando CEP...");
    let (street, neighborhood, city, state) = match api::lookup_cep(&cep).await {
        Ok(cep_data) => {
            sp.stop();
            println!(
                "  {} - {}, {}/{}",
                cep_data.logradouro.green(),
                cep_data.bairro.green(),
                cep_data.localidade,
                cep_data.uf
            );
            (
                cep_data.logradouro,
                cep_data.bairro,
                cep_data.localidade,
                cep_data.uf,
            )
        }
        Err(e) => {
            drop(sp);
            eprintln!("Erro ao buscar CEP: {}. Digite manualmente.", e);
            let street = ui::read_input("Rua: ");
            let neighborhood = ui::read_input("Bairro: ");
            let city = ui::read_input("Cidade: ");
            let state = ui::read_input("Estado (UF): ");
            (street, neighborhood, city, state)
        }
    };

    let number = ui::read_input("Número: ");
    let complement = ui::read_input("Complemento: ");
    let landmark = ui::read_input("Ponto de referência: ");

    // Offer to save (unless stateless)
    if !opts.stateless {
        let save = ui::read_input("Salvar este endereço no perfil? (S/N): ");
        if save.to_uppercase() == "S" {
            let label = ui::read_input("Nome para o endereço (ex: Casa, Trabalho): ");
            let mut config = config::load_user_config(opts).unwrap_or_default();
            config.addresses.push(SavedAddress {
                label,
                cep: cep.clone(),
                street: street.clone(),
                number: number.clone(),
                complement: complement.clone(),
                neighborhood: neighborhood.clone(),
                city: city.clone(),
                state: state.clone(),
                landmark: landmark.clone(),
                unidade_padrao: None,
            });
            let new_idx = config.addresses.len() - 1;
            let set_default = ui::read_input("Definir como endereço padrão? (S/N): ");
            if set_default.to_uppercase() == "S" {
                config.endereco_padrao = Some(new_idx);
            }
            config::save_user_config(&config, opts);
            println!("{}", "Endereço salvo!".green());
        }
    }

    let clean_cep = cep.replace('-', "");
    (
        street,
        number,
        complement,
        neighborhood,
        city,
        state,
        clean_cep,
        landmark,
    )
}

async fn ensure_client_id(
    ctx: &api::ApiContext,
    name: &str,
    phone: &str,
    user_config: &Option<UserConfig>,
    opts: &AppOptions,
) {
    if name.is_empty() || phone.is_empty() {
        return;
    }

    // Skip if already have client_id for the same name+phone
    if let Some(cfg) = user_config {
        if cfg.client_id.is_some() && cfg.name == name && cfg.phone == phone {
            return;
        }
    }

    let sp = ui::Spinner::new("Registrando cliente...");
    match api::register_client(ctx, name, phone).await {
        Ok(result) => {
            sp.stop();
            if !opts.stateless {
                let mut cfg = user_config.clone().unwrap_or_default();
                cfg.client_id = Some(result.client_id);
                if result.token.is_some() {
                    cfg.auth_token = result.token;
                }
                if cfg.name.is_empty() {
                    cfg.name = name.to_string();
                }
                if cfg.phone.is_empty() {
                    cfg.phone = phone.to_string();
                }
                config::save_user_config(&cfg, opts);
            }
        }
        Err(e) => {
            drop(sp);
            eprintln!(
                "{}",
                format!("Aviso: não foi possível obter o ID do cliente: {}", e).yellow()
            );
        }
    }
}

fn list_rewards() {
    let rewards = config::get_loyalty_rewards();
    println!("\n{}", "🏆 --- PROGRAMA DE FIDELIDADE ---".yellow().bold());

    for r in rewards {
        if r.active {
            let kind_display = match r.kind.as_str() {
                "item" => "Item Grátis".cyan(),
                "discount" => "Desconto".green(),
                _ => "Benefício".white(),
            };
            println!("   ★ {} [{}]", r.name.bold(), kind_display);
        }
    }
    println!();
}
