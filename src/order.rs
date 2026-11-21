use crate::api;
use crate::config::{self, AppOptions};
use crate::menu;
use crate::models::{
    available_menu_at, CartItem, MenuCategory, MenuItem, MenuSelection, OrderAddressPayload,
    OrderClientPayload, OrderItemPayload, OrderPayload, OrderSubItemPayload, PaymentBrandPayload,
    PaymentMethod, PaymentValuePayload, SavedAddress, SubItem, Unit, UserConfig,
};
use crate::notify;
use crate::ui;
use crate::units;
use colored::*;
use qrcode::render::unicode;
use qrcode::QrCode;

pub async fn start_order_flow(opts: &AppOptions) {
    let (selected_unit, ctx) = units::select_unit_and_context(opts).await;
    let full_menu = match config::fetch_and_cache_menu(&ctx, opts).await {
        Ok(menu) => menu,
        Err(e) => {
            eprintln!(
                "{}",
                format!(
                    "Não foi possível carregar o cardápio atualizado: {}\nO pedido não pode ser feito com preços desatualizados. Tente novamente mais tarde.",
                    e
                )
                .red()
            );
            return;
        }
    };
    let menu_data = available_menu_at(&full_menu, now_local());

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
                let item = selection_to_cart_item(sel);
                total_price += item.price;

                let mut desc = item.name.green().to_string();
                if !item.flavors.is_empty() {
                    desc.push_str(&format!(" ({})", item.flavors.join(", ")));
                }
                cart.push(item);
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
        if !item.extras.is_empty() {
            let extras: Vec<String> = item
                .extras
                .iter()
                .map(|e| format!("{}x {} ({})", e.quantity, e.name, e.add_on_name))
                .collect();
            println!(
                "  Adicionais: {}",
                extras.join(", ").truecolor(150, 150, 150)
            );
        }
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
    let mut delivery_quote = calculate_delivery_quote(
        &ctx,
        &street,
        &number,
        &neighborhood,
        &city,
        &state,
        &zip_code,
    )
    .await;
    let mut delivery_fee = delivery_quote.value;
    let mut delivery_estimated_time = delivery_quote.estimated_time.unwrap_or(0);

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
        if is_pay_on_delivery_method(&payment_choice.payload_method) {
            println!(
                "   {}",
                "Este pagamento é feito na entrega do pedido.".yellow()
            );
        }
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
                let price_notices = refresh_cart_prices(&mut cart, &full_menu, now_local());
                if !price_notices.is_empty() {
                    println!(
                        "\n{}",
                        "⚠️  O cardápio mudou desde que você montou o pedido:"
                            .yellow()
                            .bold()
                    );
                    for notice in &price_notices {
                        println!("  • {}", notice.yellow());
                    }
                    if cart.is_empty() {
                        println!("{}", "Carrinho vazio. Compra cancelada.".yellow());
                        return;
                    }
                    total_price = cart.iter().map(|i| i.price).sum::<f64>();
                    grand_total = total_price + delivery_fee;
                    println!(
                        "{}",
                        "Revise o resumo atualizado antes de confirmar.".yellow()
                    );
                    continue;
                }
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
                                let current_menu = available_menu_at(&full_menu, now_local());
                                if let Some(sel) = menu::browse_menu_select(&current_menu) {
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

                        delivery_quote = calculate_delivery_quote(
                            &ctx,
                            &street,
                            &number,
                            &neighborhood,
                            &city,
                            &state,
                            &zip_code,
                        )
                        .await;
                        delivery_fee = delivery_quote.value;
                        delivery_estimated_time = delivery_quote.estimated_time.unwrap_or(0);
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
    let mut order_items: Vec<OrderItemPayload> = Vec::with_capacity(cart.len());
    for item in &cart {
        let Some(menu_item) = find_menu_item(&full_menu, item) else {
            eprintln!(
                "{}",
                format!(
                    "{} não está mais no cardápio. Pedido não enviado.",
                    item.name
                )
                .red()
            );
            return;
        };
        let subitems = build_order_subitems(item, menu_item);
        order_items.push(OrderItemPayload {
            item_id: item.item_id,
            kind: "regular_item".to_string(),
            name: item.name.clone(),
            custom_code: item.custom_code.clone(),
            category_id: item.category_id,
            category_name: item.category_name.clone(),
            quantity: 1,
            observation: observation.clone(),
            unit_price: item.base_price,
            price: item.price,
            price_without_discounts: item.price_without_discounts,
            print_area_id: item.print_area_id,
            second_print_area_id: item.second_print_area_id,
            order_subitems_attributes: subitems,
        });
    }

    let final_value = format!("{:.2}", grand_total);
    let clean_phone: String = customer_phone
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect();
    let estimated_time =
        compute_order_estimated_time(selected_unit.preparation_time, delivery_estimated_time);

    let payload = OrderPayload {
        final_value,
        delivery_fee,
        delivery_man_fee: None,
        additional_fee: None,
        estimated_time,
        custom_fields_data: "[]".to_string(),
        company_id: selected_unit.id,
        confirmation: false,
        order_type: "delivery".to_string(),
        payment_values_attributes: vec![PaymentValuePayload {
            id: payment_choice.id,
            name: payment_choice.display_name.clone(),
            fixed_fee: payment_choice.fixed_fee,
            percentual_fee: payment_choice.percentual_fee,
            available_on_menu: payment_choice.available_on_menu,
            available_for: payment_choice.available_for.clone(),
            available_order_timings: payment_choice.available_order_timings.clone(),
            allow_on_customer_first_order: payment_choice.allow_on_customer_first_order,
            online_payment_provider: payment_choice.online_payment_provider.clone(),
            kind: payment_choice.kind.clone(),
            brands: payment_choice.brands.clone(),
            payment_method_id: payment_choice.id,
            payment_method: payment_choice.payload_method.clone(),
            payment_method_brand_id: payment_choice.default_brand_id,
            payment_fee: payment_choice.payment_fee,
            total: grand_total,
        }],
        scheduled_date: None,
        scheduled_period: None,
        earned_points: points_earned,
        sales_channel: "catalog".to_string(),
        customer_origin: None,
        diswpp_message_id: None,
        invoice_document: None,
        client_id,
        client: OrderClientPayload {
            name: customer_name.clone(),
            ddi: 55,
            telephone: clean_phone,
        },
        delivery_address: OrderAddressPayload {
            street,
            neighborhood,
            address_complement: complement,
            house_number: number,
            city,
            state,
            landmark,
            latitude: None,
            longitude: None,
            zip_code,
        },
        benefits: vec![],
        order_items,
    };

    // Submit order
    let sp = ui::Spinner::new("Enviando pedido...");
    match api::submit_order(&ctx, &payload, None).await {
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
            println!(
                "⏱️  Tempo estimado: {} minutos",
                format!("{}", estimated_time).green()
            );
            if is_pix_method(&payment_choice.payload_method) {
                match api::fetch_order_detail(&ctx, &order.uid).await {
                    Ok(detail) => print_pix_payment(&detail),
                    Err(e) => eprintln!("Não foi possível carregar os dados do PIX: {}", e),
                }
            }
            let notifications_enabled =
                config::load_user_config(opts).map_or(true, |cfg| cfg.notifications_enabled());
            if notifications_enabled {
                match notify::spawn_background_watcher(
                    &ctx,
                    &order.uid,
                    order.order_number,
                    &order.status,
                ) {
                    Ok(()) => println!(
                        "🔔 Você será avisado a cada mudança de status (desative em: {})",
                        "drpizza perfil --edit".cyan()
                    ),
                    Err(e) => eprintln!("Não foi possível iniciar as notificações: {}", e),
                }
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
    let client_result = match api::find_client_by_phone(ctx, phone).await {
        Ok(result) => Ok(result),
        Err(_) => api::register_client(ctx, name, phone).await,
    };

    match client_result {
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
    id: Option<u64>,
    payload_method: String,
    display_name: String,
    kind: String,
    fixed_fee: Option<f64>,
    percentual_fee: Option<f64>,
    available_on_menu: bool,
    available_for: Vec<String>,
    available_order_timings: Vec<String>,
    allow_on_customer_first_order: bool,
    online_payment_provider: Option<String>,
    payment_fee: Option<f64>,
    brands: Vec<PaymentBrandPayload>,
    default_brand_id: Option<u64>,
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
        let delivery_note = if is_pay_on_delivery_method(&choice.payload_method) {
            " (pago na entrega)".bright_black().to_string()
        } else {
            String::new()
        };
        println!(
            "  [{}] {}{}",
            (i + 1).to_string().cyan(),
            choice.display_name,
            delivery_note
        );
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
                if is_pay_on_delivery_method(&choice.payload_method) {
                    println!(
                        "{}",
                        "  Este pagamento é feito na entrega do pedido.".yellow()
                    );
                }
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
        "pix_auto" => "PIX".to_string(),
        "meal_voucher" => "Vale Refeição".to_string(),
        other => other.to_string(),
    }
}

fn payment_choice_from(pm: &PaymentMethod) -> PaymentChoice {
    let raw_method = pm.method.as_deref().unwrap_or("").trim();
    let raw_name = pm.name.as_deref().unwrap_or("").trim();

    let payload_method = if !raw_method.is_empty() {
        raw_method.to_string()
    } else {
        infer_payment_method_from_name(raw_name)
    };

    let display_name = if !raw_name.is_empty() {
        raw_name.to_string()
    } else {
        format_payment_name(&payload_method)
    };

    let brands = if pm.brands.is_empty() {
        vec![PaymentBrandPayload {
            id: None,
            name: None,
            kind: None,
            image_key: None,
            system_default: true,
        }]
    } else {
        pm.brands.iter().map(PaymentBrandPayload::from).collect()
    };

    let kind = pm.kind.clone().unwrap_or_else(|| payload_method.clone());

    PaymentChoice {
        id: pm.id,
        payload_method,
        display_name,
        kind,
        fixed_fee: pm.fixed_fee,
        percentual_fee: pm.percentual_fee,
        available_on_menu: pm.available_on_menu.unwrap_or(true),
        available_for: pm.available_for.clone(),
        available_order_timings: pm.available_order_timings.clone(),
        allow_on_customer_first_order: pm.allow_on_customer_first_order.unwrap_or(true),
        online_payment_provider: pm.online_payment_provider.clone(),
        payment_fee: pm.payment_fee,
        default_brand_id: brands.iter().find_map(|brand| brand.id),
        brands,
    }
}

fn infer_payment_method_from_name(name: &str) -> String {
    let n = name.to_lowercase();
    if n.contains("pix") && (n.contains("auto") || n.contains("autom")) {
        "pix_auto".to_string()
    } else if n.contains("pix") {
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

fn is_pix_method(method: &str) -> bool {
    method.to_lowercase().contains("pix")
}

fn is_pay_on_delivery_method(method: &str) -> bool {
    matches!(
        method.to_lowercase().as_str(),
        "money" | "credit_card" | "debit_card"
    )
}

fn print_pix_payment(detail: &crate::models::OrderDetail) {
    if let Some(output) = build_pix_payment_output(detail) {
        println!("\n{}", output);
    }
}

fn build_pix_payment_output(detail: &crate::models::OrderDetail) -> Option<String> {
    if let Some(payment) = detail.payment_values.iter().find(|pv| {
        pv.payment_method
            .as_deref()
            .unwrap_or_default()
            .to_lowercase()
            .contains("pix")
    }) {
        let mut lines: Vec<String> = vec!["💠 Pagamento PIX".to_string()];
        if let Some(deadline) = payment.pix_deadline_line(chrono::Local::now()) {
            lines.push(deadline);
        }

        if let Some(copy_paste) = payment.pix_qr_copy_paste.as_deref() {
            let copy_paste = copy_paste.trim();
            if !copy_paste.is_empty() {
                if let Some(qr) = render_qr_terminal(copy_paste) {
                    lines.push("📱 Escaneie o QR Code abaixo para pagar:".to_string());
                    lines.push(qr);
                }
                lines.push("📋 PIX copia e cola:".to_string());
                lines.push(copy_paste.to_string());
            }
        }

        if lines.len() > 1 {
            return Some(lines.join("\n"));
        }
    }

    None
}

fn render_qr_terminal(copy_paste: &str) -> Option<String> {
    let qr = QrCode::new(copy_paste.as_bytes()).ok()?;
    let rendered = qr.render::<unicode::Dense1x2>().quiet_zone(true).build();
    Some(rendered.trim_end().to_string())
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
    let item_price = calculate_selection_price(&sel);
    let item_price_without_discounts = calculate_selection_price_without_discounts(&sel);
    let flavor_names: Vec<String> = sel.flavors.iter().map(|f| f.name.clone()).collect();
    let flavor_ids: Vec<u32> = sel.flavors.iter().map(|f| f.id).collect();
    let crust_name = sel
        .crust
        .as_ref()
        .map(|c| c.name.clone())
        .unwrap_or_else(|| "Tradicional".to_string());
    let crust_id = sel.crust.as_ref().map(|c| c.id);
    let current_base = sel.item.get_current_price();
    let base_price = if current_base > 0.0 {
        current_base
    } else {
        0.0
    };

    CartItem {
        item_id: sel.item.id,
        name: sel.item.name.clone(),
        custom_code: sel.item.custom_code,
        print_area_id: sel.item.print_area_id,
        second_print_area_id: sel.item.second_print_area_id,
        category_id: sel.category_id,
        category_name: sel.category_name,
        flavors: flavor_names,
        flavor_ids,
        crust: crust_name,
        crust_id,
        extras: sel.extras,
        price: item_price,
        price_without_discounts: item_price_without_discounts,
        base_price,
    }
}

fn now_local() -> chrono::NaiveDateTime {
    chrono::Local::now().naive_local()
}

/// Re-checks cart items against time-bound availability and promotional prices
/// (e.g. Happy Hour ending mid-checkout). Updates/removes items in place and
/// returns one notice per change; empty means the cart is still valid.
fn refresh_cart_prices(
    cart: &mut Vec<CartItem>,
    full_menu: &[MenuCategory],
    now: chrono::NaiveDateTime,
) -> Vec<String> {
    let mut notices = Vec::new();
    cart.retain_mut(|cart_item| {
        let found = full_menu.iter().find_map(|category| {
            category
                .items
                .iter()
                .find(|i| i.id == cart_item.item_id && category.id == cart_item.category_id)
                .map(|i| (category, i))
        });
        let Some((category, menu_item)) = found else {
            notices.push(format!(
                "{} não está mais no cardápio (removido)",
                cart_item.name
            ));
            return false;
        };
        if !category.is_available_at(now) || !menu_item.is_available_at(now) {
            notices.push(format!(
                "{} não está disponível neste horário (removido)",
                cart_item.name
            ));
            return false;
        }
        let new_base = menu_item.price_at(now);
        if (new_base - cart_item.base_price).abs() > 0.005 {
            notices.push(format!(
                "{}: preço mudou de R$ {:.2} para R$ {:.2}",
                cart_item.name, cart_item.base_price, new_base
            ));
            cart_item.price += new_base - cart_item.base_price;
            cart_item.base_price = new_base;
        }
        true
    });
    notices
}

fn calculate_selection_price(sel: &MenuSelection) -> f64 {
    let mut item_price = sel.item.get_current_price() + flavors_price(sel);

    if let Some(c) = &sel.crust {
        item_price += c.price;
    }

    for extra in &sel.extras {
        item_price += extra.price * extra.quantity as f64;
    }

    item_price
}

fn calculate_selection_price_without_discounts(sel: &MenuSelection) -> f64 {
    let mut item_price = sel.item.price + flavors_price(sel);

    if let Some(c) = &sel.crust {
        item_price += c.price;
    }

    for extra in &sel.extras {
        item_price += extra.price * extra.quantity as f64;
    }

    item_price
}

fn flavors_price(sel: &MenuSelection) -> f64 {
    sel.item
        .add_ons
        .iter()
        .find(|group| group.is_flavor_group())
        .map_or(0.0, |group| group.combined_price(&sel.flavors))
}

fn find_menu_item<'a>(full_menu: &'a [MenuCategory], item: &CartItem) -> Option<&'a MenuItem> {
    full_menu
        .iter()
        .filter(|category| category.id == item.category_id)
        .flat_map(|category| category.items.iter())
        .find(|menu_item| menu_item.id == item.item_id)
}

/// Builds `order_subitems_attributes` with per-group pricing identical to the
/// cardapioweb client; the API recomputes it and rejects mismatches.
fn build_order_subitems(item: &CartItem, menu_item: &MenuItem) -> Vec<OrderSubItemPayload> {
    // Distinct (subitem_id, quantity) in selection order.
    let mut picks: Vec<(u32, u32)> = Vec::new();
    let selections = item
        .flavor_ids
        .iter()
        .map(|id| (*id, 1))
        .chain(item.crust_id.map(|id| (id, 1)))
        .chain(item.extras.iter().map(|e| (e.id, e.quantity)));
    for (id, qty) in selections {
        match picks.iter_mut().find(|(existing, _)| *existing == id) {
            Some((_, total)) => *total += qty,
            None => picks.push((id, qty)),
        }
    }

    let mut subitems = Vec::new();
    for group in &menu_item.add_ons {
        let group_picks: Vec<(&SubItem, u32)> = picks
            .iter()
            .filter_map(|(id, qty)| {
                group
                    .subitems
                    .iter()
                    .find(|sub| sub.id == *id)
                    .map(|sub| (sub, *qty))
            })
            .collect();
        if group_picks.is_empty() {
            continue;
        }
        picks.retain(|(id, _)| !group_picks.iter().any(|(sub, _)| sub.id == *id));
        let lines = group.payload_lines(&group_picks);
        for ((sub, qty), (price, total_price, count)) in group_picks.into_iter().zip(lines) {
            subitems.push(OrderSubItemPayload {
                subitem_id: sub.id,
                quantity: qty,
                price,
                total_price,
                add_on_selected_subitems_count: count,
                name: sub.name.clone(),
                custom_code: sub.custom_code.clone(),
                add_on_id: group.id,
                add_on_name: group.name.clone(),
            });
        }
    }
    subitems
}

fn compute_order_estimated_time(
    preparation_time: Option<u32>,
    delivery_estimated_time: u32,
) -> u32 {
    if delivery_estimated_time > 0 {
        preparation_time.unwrap_or(0) + delivery_estimated_time
    } else {
        preparation_time.unwrap_or(30)
    }
}

async fn calculate_delivery_quote(
    ctx: &api::ApiContext,
    street: &str,
    number: &str,
    neighborhood: &str,
    city: &str,
    state: &str,
    zip_code: &str,
) -> api::DeliveryQuote {
    let sp = ui::Spinner::new("Calculando frete...");
    match api::calculate_delivery_tax(ctx, street, number, neighborhood, city, state, zip_code)
        .await
    {
        Ok(quote) => {
            sp.stop();
            println!(
                "🚚 Taxa de entrega: {}",
                format!("R$ {:.2}", quote.value).yellow()
            );
            quote
        }
        Err(e) => {
            drop(sp);
            eprintln!("Erro ao calcular taxa de entrega: {}", e);
            api::DeliveryQuote {
                value: 0.0,
                estimated_time: None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_order_subitems, build_pix_payment_output, calculate_selection_price,
        calculate_selection_price_without_discounts, compute_order_estimated_time,
        refresh_cart_prices,
    };
    use crate::models::{
        available_menu_at, AddOnGroup, AllowedTime, CartItem, ComboStep, MenuCategory, MenuItem,
        MenuSelection, OrderDetail, OrderSubItemPayload, PaymentValue, PromotionalPriceSchedule,
        SelectedSubItem, SubItem,
    };
    use chrono::NaiveDate;

    fn at(y: i32, m: u32, d: u32, h: u32, min: u32) -> chrono::NaiveDateTime {
        NaiveDate::from_ymd_opt(y, m, d)
            .and_then(|date| date.and_hms_opt(h, min, 0))
            .expect("data valida")
    }

    /// Mirrors the API's "HAPPY HOUR (16h às 18h)" category.
    fn happy_hour_menu() -> Vec<MenuCategory> {
        let mut item = sample_selection().item;
        item.id = 5528927;
        item.name = "Pizza Grande".to_string();
        item.price = 72.9;
        item.promotional_price = Some(49.9);
        item.promotional_price_schedules = vec![PromotionalPriceSchedule {
            day: "friday".to_string(),
            start: Some("16:00".to_string()),
            end: Some("18:00".to_string()),
        }];
        vec![MenuCategory {
            id: 665244,
            name: "HAPPY HOUR (16h às 18h)".to_string(),
            description: None,
            items: vec![item],
            allowed_times: vec![AllowedTime {
                weekday: "friday".to_string(),
                start_at: "16:00".to_string(),
                end_at: "18:00".to_string(),
            }],
        }]
    }

    fn flavor_selection(base: f64, calc: &str, flavor_prices: &[f64]) -> MenuSelection {
        let flavors: Vec<SubItem> = flavor_prices
            .iter()
            .enumerate()
            .map(|(i, price)| SubItem {
                id: i as u32 + 1,
                name: format!("Sabor {}", i + 1),
                custom_code: None,
                price: *price,
            })
            .collect();
        let mut sel = sample_selection();
        sel.item.price = base;
        sel.item.promotional_price = None;
        sel.item.promotional_price_active = false;
        sel.item.add_ons = vec![AddOnGroup {
            id: 1,
            name: "Sabores Pizza - 2 Sabores".to_string(),
            subitems: flavors.clone(),
            price_calculation_type: Some(calc.to_string()),
            minimum_price: None,
        }];
        sel.flavors = flavors;
        sel.crust = None;
        sel.extras = vec![];
        sel
    }

    #[test]
    fn flavor_prices_follow_price_calculation_type() {
        // "Grande mais pedidas": base 65.90 + SUM(Pepperoni 5.00, Frango 5.50).
        let sum = flavor_selection(65.9, "SUM", &[5.0, 5.5]);
        assert!((calculate_selection_price(&sum) - 76.4).abs() < 1e-9);
        // "Pizza Grande - até 2 Sabores": base 0 + MEAN(82.90, 84.90).
        let mean = flavor_selection(0.0, "MEAN", &[82.9, 84.9]);
        assert!((calculate_selection_price(&mean) - 83.9).abs() < 1e-9);
        let max = flavor_selection(0.0, "MAX", &[82.9, 84.9]);
        assert!((calculate_selection_price(&max) - 84.9).abs() < 1e-9);
    }

    #[test]
    fn mean_flavor_subitems_split_group_price_like_web_client() {
        // Failing order: Pizza Grande 2 sabores (MEAN), Frango c/ cream cheese 86.90
        // + Pepperoni 82.90, borda tradicional (SUM, 0.00).
        let mut sel = flavor_selection(0.0, "MEAN", &[86.9, 82.9]);
        let crust = SubItem {
            id: 99,
            name: "Borda tradicional".to_string(),
            custom_code: None,
            price: 0.0,
        };
        sel.item.add_ons.push(AddOnGroup {
            id: 2,
            name: "Que tal uma borda Recheada?".to_string(),
            subitems: vec![crust.clone()],
            price_calculation_type: Some("SUM".to_string()),
            minimum_price: None,
        });
        sel.crust = Some(crust);
        let menu_item = sel.item.clone();
        let cart_item = super::selection_to_cart_item(sel);
        assert!((cart_item.price - 84.9).abs() < 1e-9);
        assert_eq!(cart_item.base_price, 0.0);

        let subitems: Vec<OrderSubItemPayload> = build_order_subitems(&cart_item, &menu_item);
        assert_eq!(subitems.len(), 3);
        for flavor in &subitems[..2] {
            assert_eq!(flavor.add_on_id, 1);
            assert!((flavor.total_price - 42.45).abs() < 1e-9);
            assert!((flavor.price - 42.45).abs() < 1e-9);
            assert_eq!(flavor.add_on_selected_subitems_count, Some(2));
        }
        assert_eq!(subitems[2].add_on_id, 2);
        assert_eq!(subitems[2].add_on_selected_subitems_count, None);
        let sum: f64 = subitems.iter().map(|s| s.total_price).sum();
        assert!((cart_item.base_price + sum - cart_item.price).abs() < 1e-9);
    }

    #[test]
    fn display_price_matches_web_catalog_rules() {
        let at_1730 = at(2026, 11, 20, 17, 30);
        // Flavor-priced pizza: base 0 + flavor group minimum.
        let mut sel = flavor_selection(0.0, "MEAN", &[82.9, 84.9]);
        sel.item.add_ons[0].minimum_price = Some(72.9);
        let price = sel.item.display_price_at(at_1730);
        assert!(price.starts_at && price.original.is_none());
        assert!((price.value - 72.9).abs() < 1e-9);

        // Active promotion keeps the old price for display.
        let promo = &happy_hour_menu()[0].items[0];
        let price = promo.display_price_at(at_1730);
        assert_eq!((price.value, price.original), (49.9, Some(72.9)));

        // Combos show the sum of their steps.
        let mut combo = sample_selection().item;
        combo.kind = "combo".to_string();
        combo.combo_steps = vec![
            ComboStep { price: Some(89.9) },
            ComboStep { price: Some(10.0) },
        ];
        assert!((combo.display_price_at(at_1730).value - 99.9).abs() < 1e-9);
    }

    #[test]
    fn pix_deadline_shows_time_left_or_expired() {
        let payment = PaymentValue {
            total: 10.0,
            payment_type: None,
            payment_method: Some("pix_auto".to_string()),
            status: None,
            pix_qr_image: None,
            pix_qr_copy_paste: None,
            pix_expires_at: Some("2026-11-21T19:42:00-03:00".to_string()),
        };
        let now = |h, m| {
            chrono::DateTime::parse_from_rfc3339(&format!("2026-11-21T{:02}:{:02}:00-03:00", h, m))
                .expect("data valida")
                .with_timezone(&chrono::Local)
        };
        let line = payment
            .pix_deadline_line(now(19, 12))
            .expect("linha do pix");
        assert!(line.contains("(30 min)"), "{}", line);
        assert_eq!(
            payment.pix_deadline_line(now(19, 50)).as_deref(),
            Some("⌛ Prazo do PIX expirou")
        );
        let no_deadline = PaymentValue {
            pix_expires_at: None,
            ..payment
        };
        assert!(no_deadline.pix_deadline_line(now(19, 12)).is_none());
    }

    #[test]
    fn promotional_price_respects_schedule() {
        let item = &happy_hour_menu()[0].items[0];
        // 2026-11-20 is a Friday.
        assert_eq!(item.price_at(at(2026, 11, 20, 17, 30)), 49.9);
        assert_eq!(item.price_at(at(2026, 11, 20, 18, 0)), 49.9);
        assert_eq!(item.price_at(at(2026, 11, 20, 18, 46)), 72.9);
        assert_eq!(item.price_at(at(2026, 11, 21, 17, 0)), 72.9);
    }

    #[test]
    fn time_restricted_category_is_hidden_outside_window() {
        let menu = happy_hour_menu();
        assert_eq!(available_menu_at(&menu, at(2026, 11, 20, 17, 0)).len(), 1);
        assert!(available_menu_at(&menu, at(2026, 11, 20, 18, 46)).is_empty());
        assert!(available_menu_at(&menu, at(2026, 11, 21, 17, 0)).is_empty());
    }

    #[test]
    fn refresh_cart_drops_items_whose_window_closed_mid_checkout() {
        let menu = happy_hour_menu();
        let mut cart = vec![CartItem {
            item_id: 5528927,
            name: "Pizza Grande".to_string(),
            custom_code: None,
            print_area_id: None,
            second_print_area_id: None,
            category_id: 665244,
            category_name: "HAPPY HOUR (16h às 18h)".to_string(),
            flavors: vec![],
            flavor_ids: vec![],
            crust: "Tradicional".to_string(),
            crust_id: None,
            extras: vec![],
            price: 49.9,
            price_without_discounts: 72.9,
            base_price: 49.9,
        }];

        assert!(refresh_cart_prices(&mut cart, &menu, at(2026, 11, 20, 17, 59)).is_empty());
        let notices = refresh_cart_prices(&mut cart, &menu, at(2026, 11, 20, 18, 46));
        assert_eq!(notices.len(), 1);
        assert!(cart.is_empty());
    }

    #[test]
    fn refresh_cart_updates_price_when_only_promotion_expires() {
        let mut menu = happy_hour_menu();
        menu[0].allowed_times.clear();
        let mut cart = vec![CartItem {
            item_id: 5528927,
            name: "Pizza Grande".to_string(),
            custom_code: None,
            print_area_id: None,
            second_print_area_id: None,
            category_id: 665244,
            category_name: String::new(),
            flavors: vec![],
            flavor_ids: vec![],
            crust: "Borda".to_string(),
            crust_id: None,
            extras: vec![],
            price: 49.9 + 15.9,
            price_without_discounts: 72.9 + 15.9,
            base_price: 49.9,
        }];

        let notices = refresh_cart_prices(&mut cart, &menu, at(2026, 11, 20, 18, 46));
        assert_eq!(notices.len(), 1);
        assert!((cart[0].price - (72.9 + 15.9)).abs() < 1e-9);
        assert_eq!(cart[0].base_price, 72.9);
    }

    fn sample_selection() -> MenuSelection {
        MenuSelection {
            item: MenuItem {
                id: 4013811,
                name: "6 ANOS DR PIZZA - Escolha até 2 Sabores".to_string(),
                custom_code: Some("23097221".to_string()),
                description: None,
                price: 75.9,
                promotional_price: Some(39.9),
                promotional_price_active: true,
                kind: "regular_item".to_string(),
                print_area_id: Some(16062),
                second_print_area_id: None,
                add_ons: vec![],
                allowed_times: vec![],
                promotional_price_schedules: vec![],
                combo_steps: vec![],
            },
            category_id: 470669,
            category_name: "6 ANOS DO DOUTOR".to_string(),
            flavors: vec![
                SubItem {
                    id: 2118968,
                    name: "Frango com Catupiry".to_string(),
                    custom_code: Some("x.17744752".to_string()),
                    price: 0.0,
                },
                SubItem {
                    id: 2118969,
                    name: "Pepperoni".to_string(),
                    custom_code: Some("x.17744753".to_string()),
                    price: 0.0,
                },
            ],
            crust: Some(SubItem {
                id: 866257,
                name: "Borda tradicional".to_string(),
                custom_code: Some("x.1374488".to_string()),
                price: 0.0,
            }),
            extras: vec![SelectedSubItem {
                id: 866319,
                name: "ENVIAR SACHÊS".to_string(),
                price: 0.0,
                quantity: 2,
                add_on_name: "ADICIONAIS".to_string(),
            }],
        }
    }

    #[test]
    fn selection_price_without_discounts_uses_base_price_not_promotional_price() {
        let selection = sample_selection();
        assert_eq!(
            calculate_selection_price_without_discounts(&selection),
            75.9
        );
    }

    #[test]
    fn estimated_time_adds_delivery_time_to_preparation_time() {
        assert_eq!(compute_order_estimated_time(Some(50), 30), 80);
        assert_eq!(compute_order_estimated_time(None, 30), 30);
        assert_eq!(compute_order_estimated_time(Some(50), 0), 50);
        assert_eq!(compute_order_estimated_time(None, 0), 30);
    }

    #[test]
    fn pix_payment_output_contains_terminal_qr_for_copy_paste() {
        let detail = OrderDetail {
            id: 1,
            uid: "pix-order-uid".to_string(),
            order_number: 1234,
            status: "pending_online_payment".to_string(),
            order_type: Some("delivery".to_string()),
            delivery_fee: Some(8.5),
            final_value: 48.4,
            earned_points: Some(48),
            observation: None,
            created_at: "2026-04-26T10:20:30.000-03:00".to_string(),
            order_items: vec![],
            delivery_address: None,
            payment_values: vec![PaymentValue {
                total: 48.4,
                payment_type: Some("online".to_string()),
                payment_method: Some("pix_auto".to_string()),
                status: Some("pending".to_string()),
                pix_qr_image: Some("https://example.test/pix.png".to_string()),
                pix_qr_copy_paste: Some(
                    "00020101021226930014BR.GOV.BCB.PIX2571pix.example/charge/12345".to_string(),
                ),
                pix_expires_at: None,
            }],
            status_changes: vec![],
            client: None,
        };

        let output = build_pix_payment_output(&detail).expect("saida pix deveria existir");
        println!("\n{}", output);
        assert!(output.contains("💠 Pagamento PIX"));
        assert!(output.contains("Escaneie o QR Code"));
        assert!(output.contains("PIX copia e cola"));
        assert!(output.contains("00020101021226930014BR.GOV.BCB.PIX"));
        assert!(
            output.contains('█') || output.contains('▀') || output.contains('▄'),
            "saida deveria conter blocos unicode do QR"
        );
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
