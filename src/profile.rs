use crate::api;
use crate::config::{self, AppOptions};
use crate::ui;
use crate::units;
use colored::*;

pub async fn show_profile(opts: &AppOptions, edit: bool) {
    if opts.stateless {
        println!(
            "{}",
            "Modo anônimo ativo. Nenhum perfil disponível.".yellow()
        );
        return;
    }

    if edit {
        edit_profile(opts).await;
    } else {
        display_profile(opts);
    }
}

fn display_profile(opts: &AppOptions) {
    match config::load_user_config(opts) {
        Some(config) => {
            println!("\n{}", "👤 --- PERFIL ---".cyan().bold());
            println!(
                "  Nome:      {}",
                if config.name.is_empty() {
                    "não definido".italic().to_string()
                } else {
                    config.name.bold().to_string()
                }
            );
            println!(
                "  Telefone:  {}",
                if config.phone.is_empty() {
                    "não definido".to_string()
                } else {
                    ui::format_phone(&config.phone)
                }
            );
            if let Some(cid) = config.client_id {
                println!("  Client ID: {}", cid);
            }

            if !config.addresses.is_empty() {
                println!(
                    "\n  {} ({} endereço(s) salvo(s))",
                    "Endereços:".yellow(),
                    config.addresses.len()
                );
                for (i, addr) in config.addresses.iter().enumerate() {
                    let default_marker = if config.endereco_padrao == Some(i) {
                        " ★"
                    } else {
                        ""
                    };
                    let unit_info = if let Some(uid) = addr.unidade_padrao {
                        format!(" [unidade: {}]", uid)
                    } else {
                        String::new()
                    };
                    println!(
                        "  [{}] {}{}{} - {}, {} - {}, {} - {}",
                        i + 1,
                        addr.label.bold(),
                        default_marker.yellow(),
                        unit_info.bright_black(),
                        addr.street,
                        addr.number,
                        addr.neighborhood,
                        addr.city,
                        addr.state
                    );
                }
                println!(
                    "\n  Use {} para gerenciar endereços.",
                    "drpizza enderecos".cyan()
                );
            } else {
                println!(
                    "\n  Nenhum endereço salvo. Use {} para adicionar.",
                    "drpizza enderecos".cyan()
                );
            }
            println!();
        }
        None => {
            println!(
                "\nNenhum perfil encontrado. Use {} para criar um.",
                "drpizza perfil --edit".cyan()
            );
        }
    }
}

async fn edit_profile(opts: &AppOptions) {
    let mut config = config::load_user_config(opts).unwrap_or_default();

    println!("\n{}", "✏️  --- EDITAR PERFIL ---".cyan().bold());

    let old_name = config.name.clone();
    let old_phone = config.phone.clone();

    let name = ui::read_input(&format!(
        "Nome [{}]: ",
        if config.name.is_empty() {
            "vazio"
        } else {
            &config.name
        }
    ));
    if !name.is_empty() {
        config.name = name;
    }

    let phone = ui::read_input(&format!(
        "Telefone [{}]: ",
        if config.phone.is_empty() {
            "vazio"
        } else {
            &config.phone
        }
    ));
    if !phone.is_empty() {
        config.phone = phone;
    }

    config::save_user_config(&config, opts);
    println!("{}", "Perfil salvo com sucesso!".green().bold());

    // Re-fetch client_id if missing or if name/phone changed
    let data_changed = config.name != old_name || config.phone != old_phone;
    let needs_client_id = config.client_id.is_none() || data_changed;

    if needs_client_id && !config.name.is_empty() && !config.phone.is_empty() {
        let sp = ui::Spinner::new("Carregando unidades...");
        if let Ok(all_units) = api::fetch_units().await {
            sp.stop();
            let unit_id = units::default_unit_id_for_config(&config, &all_units);
            if let Some(unit) = all_units.iter().find(|u| u.id == unit_id) {
                let ctx = api::ApiContext::from_unit(unit);
                let sp2 = ui::Spinner::new("Registrando cliente...");
                match api::register_client(&ctx, &config.name, &config.phone).await {
                    Ok(result) => {
                        sp2.stop();
                        config.client_id = Some(result.client_id);
                        if result.token.is_some() {
                            config.auth_token = result.token;
                        }
                        config::save_user_config(&config, opts);
                        println!("Client ID obtido: {}", result.client_id.to_string().cyan());
                    }
                    Err(e) => {
                        drop(sp2);
                        eprintln!(
                            "{}",
                            format!("Aviso: não foi possível obter o ID do cliente: {}", e)
                                .yellow()
                        );
                    }
                }
            }
        } else {
            drop(sp);
        }
    }
}
