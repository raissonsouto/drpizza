use crate::api;
use crate::config::{self, AppOptions};
use crate::orders;
use crate::ui;
use crate::units;
use colored::*;

pub async fn show_points(opts: &AppOptions) {
    let (client_id, auth_token, auth_password) = match orders::get_client_info(opts) {
        Some(info) => info,
        None => return,
    };

    let (_unit, ctx) = units::select_unit_and_context(opts).await;

    let sp = ui::Spinner::new("Buscando pedidos para calcular pontos...");
    let result = orders::fetch_all_orders(
        &ctx,
        opts,
        client_id,
        100,
        auth_token.as_deref(),
        auth_password.as_deref(),
    )
    .await;
    sp.stop();

    if result.orders.is_empty() && result.had_error {
        println!(
            "{}",
            "Não foi possível calcular seus pontos agora (erro de autenticação/API).".red()
        );
        return;
    }

    let mut total_points: u64 = 0;
    let mut detailed_orders: usize = 0;
    let mut failed_details: usize = 0;

    let sp2 = ui::Spinner::new("Somando pontos dos pedidos...");
    for order in &result.orders {
        match api::fetch_order_detail(&ctx, &order.uid).await {
            Ok(detail) => {
                total_points += detail.earned_points.unwrap_or(0);
                detailed_orders += 1;
            }
            Err(_) => failed_details += 1,
        }
    }
    sp2.stop();

    println!("\n{}", "⭐ --- PONTOS DR. PIZZA ---".yellow().bold());
    println!(
        "  Pontos acumulados: {}",
        total_points.to_string().green().bold()
    );
    println!(
        "  Pedidos considerados: {}",
        detailed_orders.to_string().cyan()
    );
    if failed_details > 0 {
        println!(
            "  {}",
            format!(
                "Aviso: {} pedido(s) não puderam ser detalhados para cálculo de pontos.",
                failed_details
            )
            .yellow()
        );
    }

    let rewards = config::get_loyalty_rewards();
    let active_rewards: Vec<_> = rewards.into_iter().filter(|r| r.active).collect();

    if active_rewards.is_empty() {
        println!("\nNenhum benefício ativo de fidelidade no momento.");
        return;
    }

    println!("\n{}", "🎁 Benefícios disponíveis".yellow().bold());
    for reward in active_rewards {
        let required = points_required(&reward);
        let status = match required {
            Some(req) if total_points >= req => "[DISPONÍVEL]".green().bold().to_string(),
            Some(req) => format!("[faltam {}]", req - total_points)
                .yellow()
                .to_string(),
            None => "[consulte regras na loja]".bright_black().to_string(),
        };
        let requirement_text = match required {
            Some(req) => format!("requer {} pontos", req),
            None => "pontuação não informada".to_string(),
        };

        println!("  {} {} ({})", status, reward.name.bold(), requirement_text);
    }
}

fn points_required(reward: &crate::models::LoyaltyReward) -> Option<u64> {
    if let Some(required) = reward.points_quantity_required {
        return Some(required as u64);
    }

    reward.points_per_currency_unit.and_then(|value| {
        if value > 0.0 {
            Some(value.ceil() as u64)
        } else {
            None
        }
    })
}
