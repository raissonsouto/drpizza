use crate::api;
use crate::config::{self, AppOptions};
use crate::models::{SavedAddress, UserConfig};
use crate::ui;
use colored::*;

pub async fn manage_addresses(
    opts: &AppOptions,
    show: bool,
    default: Option<usize>,
    remove: Option<usize>,
    add: bool,
) {
    if opts.stateless {
        println!(
            "{}",
            "Modo anônimo ativo. Gerenciamento de endereços indisponível.".yellow()
        );
        return;
    }

    let mut config = config::load_user_config(opts).unwrap_or_default();

    // Flag-based operations
    if show {
        show_addresses(&config);
        return;
    }

    if let Some(idx) = default {
        set_default_address_by_index(&mut config, opts, idx);
        return;
    }

    if let Some(idx) = remove {
        remove_address_by_index(&mut config, opts, idx);
        return;
    }

    if add {
        add_address(&mut config, opts).await;
        return;
    }

    // Interactive menu (no flags)
    loop {
        println!("\n{}", "📍 --- ENDEREÇOS ---".cyan().bold());

        if config.addresses.is_empty() {
            println!("  Nenhum endereço salvo.");
        } else {
            print_address_list(&config);
        }

        println!("\n[A] Adicionar endereço");
        if !config.addresses.is_empty() {
            println!("[E] Editar endereço");
            println!("[R] Remover endereço");
            println!("[D] Definir endereço padrão");
        }
        println!("[S] Sair");

        let choice = ui::read_input("\n> ").to_uppercase();
        match choice.as_str() {
            "A" => {
                add_address(&mut config, opts).await;
            }
            "E" if !config.addresses.is_empty() => {
                edit_address(&mut config, opts).await;
            }
            "R" if !config.addresses.is_empty() => {
                remove_address_interactive(&mut config, opts);
            }
            "D" if !config.addresses.is_empty() => {
                set_default_address_interactive(&mut config, opts);
            }
            "S" => break,
            _ => println!("{}", "Opção inválida.".red()),
        }
    }
}

fn print_address_list(config: &UserConfig) {
    for (i, addr) in config.addresses.iter().enumerate() {
        let default_marker = if config.endereco_padrao == Some(i) {
            " ★ (padrão)"
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
}

fn show_addresses(config: &UserConfig) {
    println!("\n{}", "📍 --- ENDEREÇOS ---".cyan().bold());
    if config.addresses.is_empty() {
        println!("  Nenhum endereço salvo.");
    } else {
        print_address_list(config);
    }
    println!();
}

async fn add_address(config: &mut UserConfig, opts: &AppOptions) {
    let label = ui::read_input("Nome do endereço (ex: Casa, Trabalho): ");
    let cep = ui::read_input("CEP: ");

    let sp = ui::Spinner::new("Buscando CEP...");
    let (street, neighborhood, city, state) = match api::lookup_cep(&cep).await {
        Ok(cep_data) => {
            sp.stop();
            println!("  Rua:    {}", cep_data.logradouro.green());
            println!("  Bairro: {}", cep_data.bairro.green());
            println!("  Cidade: {}/{}", cep_data.localidade, cep_data.uf);
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

    config.addresses.push(SavedAddress {
        label,
        cep,
        street,
        number,
        complement,
        neighborhood,
        city,
        state,
        landmark,
        unidade_padrao: None,
    });

    let new_idx = config.addresses.len() - 1;
    if config.addresses.len() == 1 {
        config.endereco_padrao = Some(0);
    } else {
        let set_default = ui::read_input("Definir como endereço padrão? (S/N): ");
        if set_default.to_uppercase() == "S" {
            config.endereco_padrao = Some(new_idx);
        }
    }

    config::save_user_config(config, opts);
    println!("{}", "Endereço adicionado!".green());
}

async fn edit_address(config: &mut UserConfig, opts: &AppOptions) {
    let idx_str = ui::read_input("Número do endereço para editar: ");
    let idx = match idx_str.parse::<usize>() {
        Ok(i) if i >= 1 && i <= config.addresses.len() => i - 1,
        _ => {
            println!("{}", "Índice inválido.".red());
            return;
        }
    };

    let addr = &config.addresses[idx];
    println!(
        "\nEditando: {} - {}, {}",
        addr.label.bold(),
        addr.street,
        addr.number
    );
    println!("(Pressione ENTER para manter o valor atual)\n");

    let label = ui::read_input(&format!("Nome [{}]: ", addr.label));
    let cep = ui::read_input(&format!("CEP [{}]: ", addr.cep));

    let (street, neighborhood, city, state) = if !cep.is_empty() && cep != addr.cep {
        let sp = ui::Spinner::new("Buscando CEP...");
        match api::lookup_cep(&cep).await {
            Ok(cep_data) => {
                sp.stop();
                println!("  Rua:    {}", cep_data.logradouro.green());
                println!("  Bairro: {}", cep_data.bairro.green());
                println!("  Cidade: {}/{}", cep_data.localidade, cep_data.uf);
                (
                    cep_data.logradouro,
                    cep_data.bairro,
                    cep_data.localidade,
                    cep_data.uf,
                )
            }
            Err(e) => {
                drop(sp);
                eprintln!("Erro ao buscar CEP: {}", e);
                (String::new(), String::new(), String::new(), String::new())
            }
        }
    } else {
        (String::new(), String::new(), String::new(), String::new())
    };

    let number = ui::read_input(&format!("Número [{}]: ", config.addresses[idx].number));
    let complement = ui::read_input(&format!(
        "Complemento [{}]: ",
        config.addresses[idx].complement
    ));
    let landmark = ui::read_input(&format!(
        "Ponto de referência [{}]: ",
        config.addresses[idx].landmark
    ));

    let addr = &mut config.addresses[idx];
    if !label.is_empty() {
        addr.label = label;
    }
    if !cep.is_empty() {
        addr.cep = cep;
    }
    if !street.is_empty() {
        addr.street = street;
    }
    if !neighborhood.is_empty() {
        addr.neighborhood = neighborhood;
    }
    if !city.is_empty() {
        addr.city = city;
    }
    if !state.is_empty() {
        addr.state = state;
    }
    if !number.is_empty() {
        addr.number = number;
    }
    if !complement.is_empty() {
        addr.complement = complement;
    }
    if !landmark.is_empty() {
        addr.landmark = landmark;
    }

    config::save_user_config(config, opts);
    println!("{}", "Endereço atualizado!".green());
}

fn remove_address_interactive(config: &mut UserConfig, opts: &AppOptions) {
    let idx_str = ui::read_input("Número do endereço para remover: ");
    let idx = match idx_str.parse::<usize>() {
        Ok(i) if i >= 1 && i <= config.addresses.len() => i - 1,
        _ => {
            println!("{}", "Índice inválido.".red());
            return;
        }
    };

    do_remove_address(config, opts, idx);
}

fn remove_address_by_index(config: &mut UserConfig, opts: &AppOptions, idx_1based: usize) {
    if idx_1based < 1 || idx_1based > config.addresses.len() {
        println!("{}", "Índice inválido.".red());
        return;
    }
    let idx = idx_1based - 1;
    let addr_label = config.addresses[idx].label.clone();

    let confirm = ui::read_input(&format!(
        "Remover endereço '{}'? (S/N): ",
        addr_label.bold()
    ));
    if confirm.to_uppercase() != "S" {
        println!("Operação cancelada.");
        return;
    }

    do_remove_address(config, opts, idx);
}

fn do_remove_address(config: &mut UserConfig, opts: &AppOptions, idx: usize) {
    let removed = config.addresses.remove(idx);
    println!("Endereço '{}' removido.", removed.label);

    // Adjust default index
    if let Some(default) = config.endereco_padrao {
        if default == idx {
            config.endereco_padrao = if config.addresses.is_empty() {
                None
            } else {
                Some(0)
            };
        } else if default > idx {
            config.endereco_padrao = Some(default - 1);
        }
    }

    config::save_user_config(config, opts);
    println!("{}", "Endereço removido!".green());
}

fn set_default_address_interactive(config: &mut UserConfig, opts: &AppOptions) {
    let idx_str = ui::read_input("Número do endereço padrão: ");
    let idx = match idx_str.parse::<usize>() {
        Ok(i) if i >= 1 && i <= config.addresses.len() => i - 1,
        _ => {
            println!("{}", "Índice inválido.".red());
            return;
        }
    };

    do_set_default_address(config, opts, idx);
}

fn set_default_address_by_index(config: &mut UserConfig, opts: &AppOptions, idx_1based: usize) {
    if idx_1based < 1 || idx_1based > config.addresses.len() {
        println!("{}", "Índice inválido.".red());
        return;
    }
    let idx = idx_1based - 1;
    let addr_label = config.addresses[idx].label.clone();

    let confirm = ui::read_input(&format!(
        "Definir '{}' como endereço padrão? (S/N): ",
        addr_label.bold()
    ));
    if confirm.to_uppercase() != "S" {
        println!("Operação cancelada.");
        return;
    }

    do_set_default_address(config, opts, idx);
}

fn do_set_default_address(config: &mut UserConfig, opts: &AppOptions, idx: usize) {
    config.endereco_padrao = Some(idx);

    println!(
        "Bairro do endereço padrão: {}",
        config.addresses[idx].neighborhood.bold()
    );
    println!("A unidade será sugerida automaticamente com base neste bairro ao fazer pedidos.");

    config::save_user_config(config, opts);
    println!(
        "{} definido como endereço padrão!",
        config.addresses[idx].label.green().bold()
    );
}
