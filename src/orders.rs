use crate::api;
use crate::config::{self, AppOptions};
use crate::ui;
use crate::units;
use colored::*;

pub async fn show_last_order(opts: &AppOptions) {
    let (client_id, auth_token) = match get_client_info(opts) {
        Some(info) => info,
        None => return,
    };

    let (_unit, ctx) = units::select_unit_and_context(opts).await;

    let sp = ui::Spinner::new("Buscando pedidos...");
    let orders = fetch_all_orders(&ctx, client_id, 1, auth_token.as_deref()).await;
    sp.stop();

    if orders.is_empty() {
        println!("Nenhum pedido recente.");
        return;
    }

    let order = &orders[0];
    println!("\n{}", "📦 --- ÚLTIMO PEDIDO ---".yellow().bold());

    let sp2 = ui::Spinner::new("Buscando detalhes...");
    match api::fetch_order_detail(&ctx, &order.uid).await {
        Ok(detail) => {
            sp2.stop();
            print_order_detail(&detail);
        }
        Err(e) => {
            drop(sp2);
            println!(
                "  Pedido #{} - {} - {}",
                order.order_number.to_string().cyan().bold(),
                ui::translate_status(&order.status).green().bold(),
                format!("R$ {:.2}", order.final_value).green()
            );
            eprintln!("Erro ao buscar detalhes: {}", e);
        }
    }
    println!();
}

pub async fn show_order_history(opts: &AppOptions) {
    let (client_id, auth_token) = match get_client_info(opts) {
        Some(info) => info,
        None => return,
    };

    let (_unit, ctx) = units::select_unit_and_context(opts).await;

    let sp = ui::Spinner::new("Buscando pedidos...");
    let orders = fetch_all_orders(&ctx, client_id, 10, auth_token.as_deref()).await;
    sp.stop();

    {
        if orders.is_empty() {
            println!("Nenhum pedido encontrado.");
            return;
        }

        println!("\n{}", "📋 --- HISTÓRICO DE PEDIDOS ---".yellow().bold());
        println!(
            "  {:<8} {:<12} {:<25} {:<12}",
            "#".bold(),
            "VALOR".bold(),
            "STATUS".bold(),
            "DATA".bold()
        );
        println!("  {}", "-".repeat(57).bright_black());

        for order in &orders {
            let date_part = order.created_at.split('T').next().unwrap_or("");
            println!(
                "  {:<8} {:<12} {:<25} {:<12}",
                order.order_number.to_string().cyan(),
                format!("R$ {:.2}", order.final_value).green(),
                ui::translate_status(&order.status),
                date_part.bright_black()
            );
        }

        println!(
            "\n  {} pedido(s) encontrado(s).",
            orders.len().to_string().cyan()
        );

        let choice =
            ui::read_input("\nDigite o número do pedido para ver detalhes (ou ENTER para sair): ");
        if choice.is_empty() {
            return;
        }

        if let Ok(order_num) = choice.parse::<u64>() {
            if let Some(order) = orders.iter().find(|o| o.order_number == order_num) {
                let sp2 = ui::Spinner::new("Buscando detalhes...");
                match api::fetch_order_detail(&ctx, &order.uid).await {
                    Ok(detail) => {
                        sp2.stop();
                        print_order_detail(&detail);
                    }
                    Err(e) => {
                        drop(sp2);
                        eprintln!("Erro ao buscar detalhes: {}", e);
                    }
                }
            } else {
                println!("{}", "Pedido não encontrado na lista.".red());
            }
        }

        println!();
    }
}

async fn fetch_all_orders(
    ctx: &api::ApiContext,
    client_id: u64,
    closed_limit: u32,
    auth_token: Option<&str>,
) -> Vec<crate::models::PendingOrder> {
    let mut all = Vec::new();

    if let Ok(pending) = api::fetch_pending_orders(ctx, client_id).await {
        all.extend(pending);
    }

    if let Ok(closed) = api::fetch_closed_orders(ctx, client_id, closed_limit, auth_token).await {
        all.extend(closed);
    }

    all.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    all
}

fn get_client_info(opts: &AppOptions) -> Option<(u64, Option<String>)> {
    if opts.stateless {
        println!(
            "{}",
            "Modo anônimo ativo. Histórico de pedidos indisponível sem Client ID.".yellow()
        );
        return None;
    }

    let config = match config::load_user_config(opts) {
        Some(c) => c,
        None => {
            println!(
                "Perfil não encontrado. Use {} primeiro.",
                "drpizza perfil --edit".cyan()
            );
            return None;
        }
    };

    match config.client_id {
        Some(id) => Some((id, config.auth_token)),
        None => {
            println!(
                "Client ID não configurado. Faça um pedido para que o ID seja associado automaticamente."
            );
            None
        }
    }
}

fn print_order_detail(detail: &crate::models::OrderDetail) {
    println!(
        "\n{}",
        format!("📦 Pedido #{}", detail.order_number)
            .yellow()
            .bold()
    );
    println!(
        "  Status: {}",
        ui::translate_status(&detail.status).green().bold()
    );
    println!(
        "  Valor:  {}",
        format!("R$ {:.2}", detail.final_value).green()
    );

    // Items
    println!("\n  {}", "Itens:".yellow());
    for item in &detail.order_items {
        println!(
            "    • {}x {} - {}",
            item.quantity as u32,
            item.name.bold(),
            format!("R$ {:.2}", item.price).green()
        );
        for sub in &item.order_subitems {
            let addon = sub.add_on_name.as_deref().unwrap_or("");
            if sub.price > 0.0 {
                println!(
                    "      └─ {} ({}) +{}",
                    sub.name,
                    addon,
                    format!("R$ {:.2}", sub.price).green()
                );
            } else {
                println!("      └─ {} ({})", sub.name, addon);
            }
        }
    }

    // Delivery address
    if let Some(addr) = &detail.delivery_address {
        println!("\n  {}", "Endereço de entrega:".yellow());
        println!(
            "    {}, {} - {}",
            addr.street.as_deref().unwrap_or(""),
            addr.house_number.as_deref().unwrap_or(""),
            addr.neighborhood.as_deref().unwrap_or("")
        );
        println!(
            "    {}, {} - CEP {}",
            addr.city.as_deref().unwrap_or(""),
            addr.state.as_deref().unwrap_or(""),
            addr.zip_code.as_deref().unwrap_or("")
        );
        if let Some(lm) = &addr.landmark {
            if !lm.is_empty() {
                println!("    Ref: {}", lm);
            }
        }
        if let Some(comp) = &addr.address_complement {
            if !comp.is_empty() {
                println!("    Complemento: {}", comp);
            }
        }
    }

    // Delivery fee
    if let Some(fee) = detail.delivery_fee {
        println!("\n  Taxa de entrega: {}", format!("R$ {:.2}", fee).yellow());
    }

    // Payment
    if !detail.payment_values.is_empty() {
        println!("\n  {}", "Pagamento:".yellow());
        for pv in &detail.payment_values {
            let method = pv.payment_method.as_deref().unwrap_or("desconhecido");
            let status = pv.status.as_deref().unwrap_or("");
            println!(
                "    {} - {} ({})",
                format!("R$ {:.2}", pv.total).green(),
                method,
                status
            );
        }
    }

    // Timeline
    println!("\n  {}", "Timeline:".yellow());
    for sc in &detail.status_changes {
        let who = sc.user_name.as_deref().unwrap_or("");
        let time_part = sc.created_at.split('T').nth(1).unwrap_or("");
        let time_display = time_part.split('.').next().unwrap_or(time_part);
        println!(
            "    {} {} {}",
            time_display.bright_black(),
            ui::translate_status(&sc.status).bold(),
            if who.is_empty() {
                String::new()
            } else {
                format!("({})", who)
            }
            .bright_black()
        );
    }
}
