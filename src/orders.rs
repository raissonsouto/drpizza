use crate::api;
use crate::config::{self, AppOptions};
use crate::ui;
use crate::units;
use colored::*;
use std::env;

struct FetchOrdersResult {
    orders: Vec<crate::models::PendingOrder>,
    had_error: bool,
}

fn is_not_found_no_records(err: &str) -> bool {
    let lower = err.to_lowercase();
    (lower.contains("registro não encontrado") || lower.contains("registro nao encontrado"))
        || lower.contains("404")
}

pub async fn show_last_order(opts: &AppOptions) {
    let (client_id, auth_token, auth_password) = match get_client_info(opts) {
        Some(info) => info,
        None => return,
    };

    let (_unit, ctx) = units::select_unit_and_context(opts).await;

    let sp = ui::Spinner::new("Buscando pedidos...");
    let result = fetch_all_orders(
        &ctx,
        opts,
        client_id,
        1,
        auth_token.as_deref(),
        auth_password.as_deref(),
    )
    .await;
    sp.stop();

    if result.orders.is_empty() && result.had_error {
        println!(
            "{}",
            "Não foi possível consultar seu histórico agora (erro de autenticação/API).".red()
        );
        return;
    }

    if result.orders.is_empty() {
        println!("Nenhum pedido recente.");
        return;
    }

    let order = &result.orders[0];
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
    let (client_id, auth_token, auth_password) = match get_client_info(opts) {
        Some(info) => info,
        None => return,
    };

    let (_unit, ctx) = units::select_unit_and_context(opts).await;

    let sp = ui::Spinner::new("Buscando pedidos...");
    let result = fetch_all_orders(
        &ctx,
        opts,
        client_id,
        10,
        auth_token.as_deref(),
        auth_password.as_deref(),
    )
    .await;
    sp.stop();

    {
        if result.orders.is_empty() && result.had_error {
            println!(
                "{}",
                "Não foi possível consultar seu histórico agora (erro de autenticação/API).".red()
            );
            println!(
                "Atualize seu perfil com {} e tente novamente.",
                "drpizza perfil --edit".cyan()
            );
            return;
        }

        if result.orders.is_empty() {
            println!("Nenhum pedido encontrado.");
            return;
        }

        println!("\n{}", "📋 --- HISTÓRICO DE PEDIDOS ---".yellow().bold());
        println!(
            "  {:<4} {:<10} {:<12} {:<25} {:<12}",
            "IDX".bold(),
            "PEDIDO".bold(),
            "VALOR".bold(),
            "STATUS".bold(),
            "DATA".bold()
        );
        println!("  {}", "-".repeat(69).bright_black());

        for (idx, order) in result.orders.iter().enumerate() {
            let date_br = format_date_br(&order.created_at);
            println!(
                "  {:<4} {:<10} {:<12} {:<25} {:<12}",
                idx.to_string().cyan(),
                order.order_number.to_string().bright_black(),
                format!("R$ {:.2}", order.final_value).green(),
                ui::translate_status(&order.status),
                date_br.bright_black()
            );
        }

        println!(
            "\n  {} pedido(s) encontrado(s).",
            result.orders.len().to_string().cyan()
        );

        let choice = ui::read_input(
            "\nDigite o índice (IDX) do pedido para ver detalhes (ou ENTER para sair): ",
        );
        if choice.is_empty() {
            return;
        }

        if let Ok(order_idx) = choice.parse::<usize>() {
            if let Some(order) = result.orders.get(order_idx) {
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
                println!("{}", "Índice não encontrado na lista.".red());
            }
        } else {
            println!("{}", "Índice inválido.".red());
        }

        println!();
    }
}

async fn fetch_all_orders(
    ctx: &api::ApiContext,
    opts: &AppOptions,
    client_id: u64,
    closed_limit: u32,
    auth_token: Option<&str>,
    auth_password: Option<&str>,
) -> FetchOrdersResult {
    let mut all = Vec::new();
    let mut had_error = false;

    match api::fetch_pending_orders(ctx, client_id).await {
        Ok(pending) => all.extend(pending),
        Err(e) => {
            if !is_not_found_no_records(&e.to_string()) {
                had_error = true;
            }
        }
    }

    match api::fetch_closed_orders(ctx, client_id, closed_limit, auth_token).await {
        Ok(closed) => {
            all.extend(closed);
        }
        Err(e) => {
            let msg = e.to_string();
            if !is_not_found_no_records(&msg) {
                had_error = true;
            }
            let msg_lower = msg.to_lowercase();
            let token_invalid = msg_lower.contains("token inválido")
                || msg_lower.contains("token invalido")
                || msg_lower.contains("token expirado")
                || msg.contains("401");

            if token_invalid {
                if let Some(password) = auth_password {
                    eprintln!(
                        "{}",
                        "Aviso: token inválido. Renovando sessão do cliente...".yellow()
                    );
                    match api::login_client_session(ctx, client_id, password).await {
                        Ok(new_token) => {
                            save_auth_token(opts, &new_token);
                            match api::fetch_closed_orders(
                                ctx,
                                client_id,
                                closed_limit,
                                Some(&new_token),
                            )
                            .await
                            {
                                Ok(closed) => {
                                    all.extend(closed);
                                    had_error = false;
                                }
                                Err(e2) => eprintln!("Erro ao buscar pedidos fechados: {}", e2),
                            }
                        }
                        Err(e2) => {
                            eprintln!("Erro ao renovar token de sessão: {}", e2);
                        }
                    }
                } else {
                    eprintln!(
                        "{}",
                        "Token inválido e senha ausente. Configure em `drpizza perfil --edit` ou variável `DRPIZZA_AUTH_PASSWORD`.".yellow()
                    );
                }
            } else {
                if !is_not_found_no_records(&msg) {
                    eprintln!("Erro ao buscar pedidos fechados: {}", msg);
                }
            }
        }
    }

    all.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    FetchOrdersResult {
        orders: all,
        had_error,
    }
}

fn get_client_info(opts: &AppOptions) -> Option<(u64, Option<String>, Option<String>)> {
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

    let auth_password = env::var("DRPIZZA_AUTH_PASSWORD")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .or(config.auth_password);

    match config.client_id {
        Some(id) => Some((id, config.auth_token, auth_password)),
        None => {
            println!(
                "Client ID não configurado. Faça um pedido para que o ID seja associado automaticamente."
            );
            None
        }
    }
}

fn save_auth_token(opts: &AppOptions, token: &str) {
    if opts.stateless {
        return;
    }
    if let Some(mut cfg) = config::load_user_config(opts) {
        cfg.auth_token = Some(token.to_string());
        config::save_user_config(&cfg, opts);
    }
}

fn format_date_br(created_at: &str) -> String {
    let date_part = created_at.split('T').next().unwrap_or(created_at);
    let mut parts = date_part.split('-');
    let year = parts.next();
    let month = parts.next();
    let day = parts.next();

    match (day, month, year) {
        (Some(d), Some(m), Some(y)) => format!("{}/{}/{}", d, m, y),
        _ => date_part.to_string(),
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
            format!("R$ {:.2}", item.display_price()).green()
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
            if let Some(qr_image) = pv.pix_qr_image.as_deref() {
                if !qr_image.is_empty() {
                    println!("      QR Code: {}", qr_image);
                }
            }
            if let Some(copy_paste) = pv.pix_qr_copy_paste.as_deref() {
                if !copy_paste.is_empty() {
                    println!("      PIX copia e cola: {}", copy_paste);
                }
            }
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
