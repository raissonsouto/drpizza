use crate::api;
use crate::config::{self, AppOptions};
use crate::menu;
use crate::models::{
    CartItem, MenuSelection, OrderAddressPayload, OrderData, OrderItemPayload, OrderPayload,
    OrderSubItemPayload, PaymentMethod, PaymentValuePayload, SavedAddress, Unit, UserConfig,
};
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
                let flavor_ids: Vec<u32> = sel.flavors.iter().map(|f| f.id).collect();
                for f in &sel.flavors {
                    item_price += f.price;
                }
                let crust_name = sel
                    .crust
                    .as_ref()
                    .map(|c| c.name.clone())
                    .unwrap_or_else(|| "Tradicional".to_string());
                let crust_id = sel.crust.as_ref().map(|c| c.id);
                if let Some(c) = &sel.crust {
                    item_price += c.price;
                }

                cart.push(CartItem {
                    item_id: sel.item.id,
                    name: sel.item.name.clone(),
                    flavors: flavor_names.clone(),
                    flavor_ids,
                    crust: crust_name.clone(),
                    crust_id,
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
    let client_id =
        ensure_client_id(&ctx, &customer_name, &customer_phone, &user_config, opts).await;

    let client_id = match client_id {
        Some(id) => id,
        None => {
            println!(
                "{}",
                "Não foi possível obter o ID do cliente. Não é possível enviar o pedido.".red()
            );
            return;
        }
    };

    // Address selection
    let (
        mut street,
        mut number,
        mut complement,
        mut neighborhood,
        mut city,
        mut state,
        mut zip_code,
        mut landmark,
    ) = select_delivery_address(&user_config, opts).await;

    // Calculate delivery tax
    let mut delivery_fee = calculate_delivery_fee(
        &ctx,
        &street,
        &number,
        &neighborhood,
        &city,
        &state,
        &zip_code,
    )
    .await;

    let mut grand_total = total_price + delivery_fee;

    // Validate minimum order value
    if let Some(min_value) = selected_unit.minimum_order_value {
        if grand_total < min_value {
            println!(
                "{}",
                format!(
                    "Pedido mínimo é R$ {:.2}. Seu pedido: R$ {:.2}.",
                    min_value, grand_total
                )
                .red()
            );
            return;
        }
    }

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

    // Payment method selection
    let mut payment_choice = match select_payment_method(&selected_unit, grand_total) {
        Some(m) => m,
        None => {
            println!("{}", "Nenhuma forma de pagamento selecionada.".red());
            return;
        }
    };

    // Change for cash + note
    let mut change_for: Option<String> = None;
    if is_money_method(&payment_choice.payload_method) {
        let change_input = ui::read_input("Troco para quanto? (ENTER se não precisa): ");
        if !change_input.is_empty() {
            change_for = Some(change_input);
        }
    }

    let mut note = ui::read_input("Alguma observação? (ENTER para pular): ");
    let mut observation = compose_observation(change_for.as_deref(), &note);

    loop {
        println!("\n{}", "📋 RESUMO FINAL".on_white().black().bold());
        println!("{}", "-------------------".bright_black());
        for item in &cart {
            println!(
                "• {:.<40} {}",
                item.name,
                format!("R$ {:.2}", item.price).green()
            );
        }
        println!("{}", "-------------------".bright_black());
        println!(
            "📍 Endereço: {}, {} - {}, {}",
            street, number, neighborhood, city
        );
        println!("💳 Pagamento: {}", payment_choice.display_name);
        if !observation.is_empty() {
            println!("📝 Obs: {}", observation.truecolor(150, 150, 150));
        }
        println!("🚚 Entrega: {}", format!("R$ {:.2}", delivery_fee).yellow());
        println!(
            "💰 TOTAL: {}",
            format!("R$ {:.2}", grand_total).green().bold()
        );
        println!("{}", "-------------------".bright_black());

        let action =
            ui::read_input("[C] Confirmar pedido / [E] Editar opções / [X] Cancelar compra: ");
        match action.trim().to_uppercase().as_str() {
            "C" | "" => {
                if let Some(min_value) = selected_unit.minimum_order_value {
                    if grand_total < min_value {
                        println!(
                            "{}",
                            format!(
                                "Pedido mínimo é R$ {:.2}. Seu pedido: R$ {:.2}.",
                                min_value, grand_total
                            )
                            .red()
                        );
                        continue;
                    }
                }
                break;
            }
            "X" => {
                println!("{}", "Compra cancelada.".yellow());
                return;
            }
            "E" => {
                println!("  [1] Forma de pagamento");
                println!("  [2] Observação");
                println!("  [3] Troco (somente dinheiro)");
                println!("  [4] Pedido (itens)");
                println!("  [5] Endereço");
                let edit = ui::read_input("Editar opção: ");
                match edit.trim() {
                    "1" => {
                        if let Some(new_choice) = select_payment_method(&selected_unit, grand_total)
                        {
                            payment_choice = new_choice;
                            if !is_money_method(&payment_choice.payload_method) {
                                change_for = None;
                            }
                            observation = compose_observation(change_for.as_deref(), &note);
                        }
                    }
                    "2" => {
                        note = ui::read_input("Nova observação (ENTER para limpar): ");
                        observation = compose_observation(change_for.as_deref(), &note);
                    }
                    "3" => {
                        if is_money_method(&payment_choice.payload_method) {
                            let troco = ui::read_input("Troco para quanto? (ENTER para limpar): ");
                            if troco.is_empty() {
                                change_for = None;
                            } else {
                                change_for = Some(troco);
                            }
                            observation = compose_observation(change_for.as_deref(), &note);
                        } else {
                            println!(
                                "{}",
                                "Troco só se aplica para pagamento em dinheiro.".yellow()
                            );
                        }
                    }
                    "4" => {
                        println!("  [1] Adicionar item");
                        println!("  [2] Remover item");
                        let pedido_edit = ui::read_input("Editar pedido: ");
                        match pedido_edit.trim() {
                            "1" => {
                                if let Some(sel) = menu::browse_menu_select(&menu_data) {
                                    let item = selection_to_cart_item(sel);
                                    println!("{} adicionado!", item.name.clone().green().bold());
                                    cart.push(item);
                                    total_price = cart.iter().map(|i| i.price).sum::<f64>();
                                    grand_total = total_price + delivery_fee;
                                }
                            }
                            "2" => {
                                if cart.is_empty() {
                                    println!("{}", "Carrinho já está vazio.".yellow());
                                } else {
                                    println!("\nItens no carrinho:");
                                    for (i, item) in cart.iter().enumerate() {
                                        println!(
                                            "  [{}] {} ({})",
                                            i + 1,
                                            item.name.bold(),
                                            format!("R$ {:.2}", item.price).green()
                                        );
                                    }
                                    let remove = ui::read_input("Remover item número: ");
                                    if let Ok(rm_idx) = remove.parse::<usize>() {
                                        if rm_idx >= 1 && rm_idx <= cart.len() {
                                            let removed = cart.remove(rm_idx - 1);
                                            println!(
                                                "{} removido do carrinho.",
                                                removed.name.yellow()
                                            );
                                            if cart.is_empty() {
                                                println!(
                                                    "{}",
                                                    "Carrinho vazio. Compra cancelada.".yellow()
                                                );
                                                return;
                                            }
                                            total_price = cart.iter().map(|i| i.price).sum::<f64>();
                                            grand_total = total_price + delivery_fee;
                                        } else {
                                            println!("{}", "Índice inválido.".red());
                                        }
                                    } else {
                                        println!("{}", "Entrada inválida.".red());
                                    }
                                }
                            }
                            _ => println!("{}", "Opção inválida.".red()),
                        }
                    }
                    "5" => {
                        let refreshed_config = config::load_user_config(opts);
                        let new_address = select_delivery_address(&refreshed_config, opts).await;
                        street = new_address.0;
                        number = new_address.1;
                        complement = new_address.2;
                        neighborhood = new_address.3;
                        city = new_address.4;
                        state = new_address.5;
                        zip_code = new_address.6;
                        landmark = new_address.7;

                        delivery_fee = calculate_delivery_fee(
                            &ctx,
                            &street,
                            &number,
                            &neighborhood,
                            &city,
                            &state,
                            &zip_code,
                        )
                        .await;
                        total_price = cart.iter().map(|i| i.price).sum::<f64>();
                        grand_total = total_price + delivery_fee;
                    }
                    _ => println!("{}", "Opção inválida.".red()),
                }
            }
            _ => println!("{}", "Opção inválida.".red()),
        }
    }

    // Build payload
    let order_items: Vec<OrderItemPayload> = cart
        .iter()
        .map(|item| {
            let mut subitems: Vec<OrderSubItemPayload> = Vec::new();
            for &fid in &item.flavor_ids {
                subitems.push(OrderSubItemPayload {
                    subitem_id: fid,
                    price: 0.0,
                    quantity: 1,
                });
            }
            if let Some(cid) = item.crust_id {
                subitems.push(OrderSubItemPayload {
                    subitem_id: cid,
                    price: 0.0,
                    quantity: 1,
                });
            }
            OrderItemPayload {
                item_id: item.item_id,
                quantity: 1,
                price: item.price,
                order_subitems: subitems,
            }
        })
        .collect();

    let payload = OrderPayload {
        order: OrderData {
            order_type: "delivery".to_string(),
            client_id,
            observation,
            delivery_address: OrderAddressPayload {
                street,
                house_number: number,
                neighborhood,
                city,
                state,
                zip_code,
                landmark,
                address_complement: complement,
            },
            order_items,
            payment_values: vec![PaymentValuePayload {
                payment_method: payment_choice.payload_method,
                total: grand_total,
            }],
        },
    };

    // Submit order
    let sp = ui::Spinner::new("Enviando pedido...");
    match api::submit_order(&ctx, &payload).await {
        Ok(order) => {
            sp.stop();
            println!();
            println!("{}", "🎉 Pedido enviado com sucesso!".green().bold());
            println!(
                "📋 Número do pedido: {}",
                format!("#{}", order.order_number).cyan().bold()
            );
            println!(
                "📌 Status: {}",
                ui::translate_status(&order.status).yellow()
            );
            if let Some(prep_time) = selected_unit.preparation_time {
                println!(
                    "⏱️  Tempo estimado: {} minutos",
                    format!("{}", prep_time).green()
                );
            }
            println!("{}", "\n🍕 Aguarde sua pizza! Bom apetite!".green().bold());
        }
        Err(e) => {
            drop(sp);
            eprintln!("{}", format!("\nErro ao enviar pedido: {}", e).red());
            println!("Tente novamente mais tarde ou entre em contato com a loja.");
        }
    }
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
) -> Option<u64> {
    if name.is_empty() || phone.is_empty() {
        return None;
    }

    // Return existing client_id if already have one for the same name+phone
    if let Some(cfg) = user_config {
        if cfg.client_id.is_some() && cfg.name == name && cfg.phone == phone {
            return cfg.client_id;
        }
    }

    let sp = ui::Spinner::new("Registrando cliente...");
    match api::register_client(ctx, name, phone).await {
        Ok(result) => {
            sp.stop();
            let cid = result.client_id;
            if !opts.stateless {
                let mut cfg = user_config.clone().unwrap_or_default();
                cfg.client_id = Some(cid);
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
            Some(cid)
        }
        Err(e) => {
            drop(sp);
            eprintln!(
                "{}",
                format!("Aviso: não foi possível obter o ID do cliente: {}", e).yellow()
            );
            // Fall back to saved client_id if available
            user_config.as_ref().and_then(|cfg| cfg.client_id)
        }
    }
}

struct PaymentChoice {
    payload_method: String,
    display_name: String,
}

fn select_payment_method(unit: &Unit, total: f64) -> Option<PaymentChoice> {
    let active_methods: Vec<_> = unit
        .payment_methods
        .iter()
        .filter(|pm| pm.active.unwrap_or(false))
        .collect();

    if active_methods.is_empty() {
        println!(
            "{}",
            "Nenhuma forma de pagamento disponível para esta unidade.".red()
        );
        return None;
    }

    println!("\n{}", "💳 Forma de pagamento:".yellow().bold());
    for (i, pm) in active_methods.iter().enumerate() {
        let choice = payment_choice_from(pm);
        println!("  [{}] {}", (i + 1).to_string().cyan(), choice.display_name);
    }

    loop {
        if let Some(idx) = ui::read_int("Escolha: ") {
            if idx >= 1 && idx <= active_methods.len() as u32 {
                let choice = payment_choice_from(active_methods[(idx - 1) as usize]);
                println!(
                    "  Pagamento: {} para {}",
                    choice.display_name.green(),
                    format!("R$ {:.2}", total).green().bold()
                );
                return Some(choice);
            }
        }
        println!("{}", "Opção inválida.".red());
    }
}

fn format_payment_name(method: &str) -> String {
    let m = method.trim();
    if m.is_empty() {
        return "Pagamento não identificado".to_string();
    }
    match m.to_lowercase().as_str() {
        "money" => "Dinheiro".to_string(),
        "credit_card" => "Cartão de Crédito".to_string(),
        "debit_card" => "Cartão de Débito".to_string(),
        "pix" => "PIX".to_string(),
        "meal_voucher" => "Vale Refeição".to_string(),
        other => other.to_string(),
    }
}

fn payment_choice_from(pm: &PaymentMethod) -> PaymentChoice {
    let raw_method = pm.method.as_deref().unwrap_or("").trim();
    let raw_name = pm.name.as_deref().unwrap_or("").trim();

    let payload_method = if !raw_method.is_empty() {
        raw_method.to_string()
    } else if !raw_name.is_empty() {
        raw_name.to_string()
    } else {
        infer_payment_method_from_name(raw_name)
    };

    let display_name = if !raw_name.is_empty() {
        raw_name.to_string()
    } else {
        format_payment_name(&payload_method)
    };

    PaymentChoice {
        payload_method,
        display_name,
    }
}

fn infer_payment_method_from_name(name: &str) -> String {
    let n = name.to_lowercase();
    if n.contains("pix") {
        "pix".to_string()
    } else if n.contains("débito") || n.contains("debito") {
        "debit_card".to_string()
    } else if n.contains("crédito") || n.contains("credito") {
        "credit_card".to_string()
    } else if n.contains("dinheiro") {
        "money".to_string()
    } else if n.contains("vale") || n.contains("refeição") || n.contains("refeicao") {
        "meal_voucher".to_string()
    } else {
        name.trim().to_lowercase().replace(' ', "_")
    }
}

fn is_money_method(method: &str) -> bool {
    method.eq_ignore_ascii_case("money")
}

fn compose_observation(change_for: Option<&str>, note: &str) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(v) = change_for {
        if !v.trim().is_empty() {
            parts.push(format!("Troco para R$ {}", v.trim()));
        }
    }
    if !note.trim().is_empty() {
        parts.push(note.trim().to_string());
    }
    parts.join(" | ")
}

fn selection_to_cart_item(sel: MenuSelection) -> CartItem {
    let mut item_price = sel.item.get_current_price();
    let flavor_names: Vec<String> = sel.flavors.iter().map(|f| f.name.clone()).collect();
    let flavor_ids: Vec<u32> = sel.flavors.iter().map(|f| f.id).collect();
    for f in &sel.flavors {
        item_price += f.price;
    }
    let crust_name = sel
        .crust
        .as_ref()
        .map(|c| c.name.clone())
        .unwrap_or_else(|| "Tradicional".to_string());
    let crust_id = sel.crust.as_ref().map(|c| c.id);
    if let Some(c) = &sel.crust {
        item_price += c.price;
    }

    CartItem {
        item_id: sel.item.id,
        name: sel.item.name.clone(),
        flavors: flavor_names,
        flavor_ids,
        crust: crust_name,
        crust_id,
        price: item_price,
    }
}

async fn calculate_delivery_fee(
    ctx: &api::ApiContext,
    street: &str,
    number: &str,
    neighborhood: &str,
    city: &str,
    state: &str,
    zip_code: &str,
) -> f64 {
    let sp = ui::Spinner::new("Calculando frete...");
    match api::calculate_delivery_tax(ctx, street, number, neighborhood, city, state, zip_code)
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
