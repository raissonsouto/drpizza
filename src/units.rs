use crate::api::{self, ApiContext};
use crate::config::{self, AppOptions};
use crate::models::{Unit, UserConfig};
use crate::ui;
use colored::*;

const DEFAULT_UNIT_ID: u32 = 7842;

// --- Select Unit + Build Context ---

pub async fn select_unit_and_context(opts: &AppOptions) -> (Unit, ApiContext) {
    let sp = ui::Spinner::new("Carregando unidades...");
    let units = api::fetch_units()
        .await
        .expect("Falha ao carregar unidades");
    sp.stop();

    let user_config = config::load_user_config(opts);

    let selected_unit = if let Some(uid) = opts.unit_id {
        units
            .iter()
            .find(|u| u.id == uid)
            .cloned()
            .expect("Unidade não encontrada")
    } else {
        let default_id = determine_default_unit_id(&user_config, &units);
        select_with_default(&units, default_id)
    };

    // Maybe ask to save as default
    maybe_save_default_unit(&selected_unit, &user_config, opts);

    let ctx = ApiContext::from_unit(&selected_unit);
    (selected_unit, ctx)
}

pub fn default_unit_id_for_config(cfg: &UserConfig, units: &[Unit]) -> u32 {
    if let Some(idx) = cfg.endereco_padrao {
        if idx < cfg.addresses.len() {
            if let Some(id) = cfg.addresses[idx].unidade_padrao {
                return id;
            }
        }
    }
    if let Some(neighborhood) = get_default_neighborhood(cfg) {
        if let Some(unit) = find_unit_for_neighborhood(units, &neighborhood) {
            return unit.id;
        }
    }
    DEFAULT_UNIT_ID
}

fn determine_default_unit_id(user_config: &Option<UserConfig>, units: &[Unit]) -> u32 {
    if let Some(cfg) = user_config {
        if let Some(idx) = cfg.endereco_padrao {
            if idx < cfg.addresses.len() {
                if let Some(id) = cfg.addresses[idx].unidade_padrao {
                    return id;
                }
            }
        }
        if let Some(neighborhood) = get_default_neighborhood(cfg) {
            if let Some(unit) = find_unit_for_neighborhood(units, &neighborhood) {
                return unit.id;
            }
        }
    }
    DEFAULT_UNIT_ID
}

fn select_with_default(units: &[Unit], default_id: u32) -> Unit {
    if let Some(default_unit) = units.iter().find(|u| u.id == default_id) {
        loop {
            let input = ui::read_input(&format!(
                "Unidade: {} [{}] (ENTER p/ confirmar ou digite o ID): ",
                default_unit.name.bold(),
                default_unit.id.to_string().cyan()
            ));
            if input.is_empty() {
                return default_unit.clone();
            }
            if let Ok(id) = input.parse::<u32>() {
                if let Some(u) = units.iter().find(|u| u.id == id) {
                    return u.clone();
                }
                println!("{}", "Unidade não encontrada.".red());
            } else {
                println!("{}", "Entrada inválida.".red());
            }
        }
    } else {
        pick_unit_interactively(units)
    }
}

fn maybe_save_default_unit(selected: &Unit, user_config: &Option<UserConfig>, opts: &AppOptions) {
    if opts.stateless {
        return;
    }

    if let Some(cfg) = user_config {
        if let Some(idx) = cfg.endereco_padrao {
            if idx < cfg.addresses.len() {
                if cfg.addresses[idx].unidade_padrao == Some(selected.id) {
                    return;
                }
            }
        }
        if cfg.nao_perguntar_unidade {
            return;
        }
    }

    let choice =
        ui::read_input("Salvar como unidade padrão? (S)im / (N)ão / (P) Não perguntar novamente: ");
    match choice.to_uppercase().chars().next() {
        Some('S') => {
            let mut cfg = user_config.clone().unwrap_or_default();
            if let Some(idx) = cfg.endereco_padrao {
                if idx < cfg.addresses.len() {
                    cfg.addresses[idx].unidade_padrao = Some(selected.id);
                }
            }
            config::save_user_config(&cfg, opts);
            println!("{}", "Unidade salva como padrão!".green());
        }
        Some('P') => {
            let mut cfg = user_config.clone().unwrap_or_default();
            cfg.nao_perguntar_unidade = true;
            config::save_user_config(&cfg, opts);
        }
        _ => {}
    }
}

fn pick_unit_interactively(units: &[Unit]) -> Unit {
    loop {
        print_units_list(units);
        if let Some(choice) = ui::read_int("Digite o número da unidade: ") {
            if let Some(u) = units.iter().find(|u| u.id == choice) {
                return u.clone();
            }
        }
        println!("{}", "Opção inválida.".red());
    }
}

fn get_default_neighborhood(cfg: &UserConfig) -> Option<String> {
    if let Some(idx) = cfg.endereco_padrao {
        if idx < cfg.addresses.len() {
            return Some(cfg.addresses[idx].neighborhood.clone());
        }
    }
    None
}

fn find_unit_for_neighborhood<'a>(units: &'a [Unit], neighborhood: &str) -> Option<&'a Unit> {
    let needle = neighborhood.to_lowercase();
    units.iter().find(|u| {
        u.delivery_only_for_neighborhoods
            .iter()
            .any(|n| n.name.to_lowercase() == needle)
    })
}

fn unit_serves_neighborhood(u: &Unit, neighborhood: &str) -> bool {
    let needle = neighborhood.to_lowercase();
    u.delivery_only_for_neighborhoods
        .iter()
        .any(|n| n.name.to_lowercase() == needle)
}

// --- List Units ---

fn print_units_list(units: &[Unit]) {
    println!("\n{}", "📍 --- UNIDADES DISPONÍVEIS ---".red().bold());
    for u in units {
        println!(
            "[{}] {}\n    └─ {}",
            u.id.to_string().cyan(),
            u.name.bold(),
            u.street
                .as_deref()
                .unwrap_or("Endereço não disponível")
                .italic()
        );
    }
    println!();
}

pub async fn list_units(
    opts: &AppOptions,
    all: bool,
    detalhes: bool,
    set_default: Option<u32>,
    no_default: bool,
) {
    let sp = ui::Spinner::new("Carregando unidades...");
    let units = match api::fetch_units().await {
        Ok(u) => {
            sp.stop();
            u
        }
        Err(e) => {
            drop(sp);
            eprintln!("Erro ao buscar unidades: {}", e);
            return;
        }
    };

    let user_config = config::load_user_config(opts);
    let default_neighborhood = user_config
        .as_ref()
        .and_then(|cfg| get_default_neighborhood(cfg));

    // Handle --no-default: remove default unit from current address
    if no_default {
        handle_remove_default_unit(&user_config, opts);
        return;
    }

    // Handle -d <id>: set default unit for current address
    if let Some(unit_id) = set_default {
        handle_set_default_unit(unit_id, &units, &user_config, opts);
        return;
    }

    // If -u was passed
    if let Some(uid) = opts.unit_id {
        match units.iter().find(|u| u.id == uid) {
            Some(u) => {
                if detalhes {
                    print_unit_details(u, default_neighborhood.as_deref());
                } else {
                    print_unit_compact(u, default_neighborhood.as_deref());
                }
            }
            None => println!("{}", "Unidade não encontrada.".red()),
        }
        return;
    }

    // Filter units by neighborhood (unless -a or no neighborhood)
    let filtered_units: Vec<&Unit> = if !all {
        if let Some(ref dn) = default_neighborhood {
            let filtered: Vec<&Unit> = units
                .iter()
                .filter(|u| unit_serves_neighborhood(u, dn))
                .collect();
            if filtered.is_empty() {
                units.iter().collect()
            } else {
                filtered
            }
        } else {
            units.iter().collect()
        }
    } else {
        units.iter().collect()
    };

    if detalhes {
        // Detailed view for all filtered units
        for u in &filtered_units {
            print_unit_details(u, default_neighborhood.as_deref());
        }
    } else {
        // Compact listing
        print_compact_list(&filtered_units, default_neighborhood.as_deref());
    }

    if let Some(ref dn) = default_neighborhood {
        println!(
            "\n  {} Unidades que atendem o bairro {}",
            "★".yellow(),
            dn.bold()
        );
        if !all {
            println!(
                "  Use {} para ver todas as unidades.",
                "drpizza unidades -a".cyan()
            );
        }
    }
    println!();
}

fn print_compact_list(units: &[&Unit], default_neighborhood: Option<&str>) {
    let today = ui::today_weekday();
    println!("\n{}", "📍 --- UNIDADES DISPONÍVEIS ---".red().bold());

    for u in units {
        let star = if let Some(dn) = default_neighborhood {
            if unit_serves_neighborhood(u, dn) {
                format!(" {}", "★".yellow())
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        println!("\n[{}] {}{}", u.id.to_string().cyan(), u.name.bold(), star,);
        println!("    └─ {}", u.formatted_address().italic());

        // Today's hours
        if let Some(bh) = &u.business_hours {
            let today_hours = ui::get_day_hours(bh, &today);
            print!("    🕐 Hoje: ");
            match today_hours {
                Some(slots) if !slots.is_empty() => {
                    let formatted: Vec<String> = slots
                        .iter()
                        .map(|slot| {
                            if slot.len() == 2 {
                                format!("{} - {}", slot[0], slot[1])
                            } else {
                                slot.join(", ")
                            }
                        })
                        .collect();
                    println!("{}", formatted.join(" | ").green());
                }
                _ => println!("{}", "Fechado".red()),
            }
        }

        // Modalities
        if let Some(flags) = &u.flags {
            let mut modos: Vec<String> = Vec::new();
            if flags.work_with_delivery {
                match u.preparation_time {
                    Some(t) => modos.push(format!("Delivery (~{}min)", t)),
                    None => modos.push("Delivery".to_string()),
                }
            }
            if flags.work_with_pick_up_store {
                modos.push("Retirada".to_string());
            }
            if flags.work_with_onsite {
                modos.push("No local".to_string());
            }
            if !modos.is_empty() {
                println!("    📦 {}", modos.join(", ").green());
            }
        }

        // WhatsApp
        if let Some(wpp) = &u.order_whatsapp {
            if !wpp.is_empty() {
                println!("    📱 WhatsApp: {}", ui::format_phone(wpp));
            }
        }
    }
}

fn print_unit_compact(u: &Unit, default_neighborhood: Option<&str>) {
    let today = ui::today_weekday();

    let star = if let Some(dn) = default_neighborhood {
        if unit_serves_neighborhood(u, dn) {
            format!(" {}", "★".yellow())
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    println!("\n[{}] {}{}", u.id.to_string().cyan(), u.name.bold(), star,);
    println!("    └─ {}", u.formatted_address().italic());

    if let Some(bh) = &u.business_hours {
        let today_hours = ui::get_day_hours(bh, &today);
        print!("    🕐 Hoje: ");
        match today_hours {
            Some(slots) if !slots.is_empty() => {
                let formatted: Vec<String> = slots
                    .iter()
                    .map(|slot| {
                        if slot.len() == 2 {
                            format!("{} - {}", slot[0], slot[1])
                        } else {
                            slot.join(", ")
                        }
                    })
                    .collect();
                println!("{}", formatted.join(" | ").green());
            }
            _ => println!("{}", "Fechado".red()),
        }
    }

    if let Some(flags) = &u.flags {
        let mut modos: Vec<String> = Vec::new();
        if flags.work_with_delivery {
            match u.preparation_time {
                Some(t) => modos.push(format!("Delivery (~{}min)", t)),
                None => modos.push("Delivery".to_string()),
            }
        }
        if flags.work_with_pick_up_store {
            modos.push("Retirada".to_string());
        }
        if flags.work_with_onsite {
            modos.push("No local".to_string());
        }
        if !modos.is_empty() {
            println!("    📦 {}", modos.join(", ").green());
        }
    }

    if let Some(wpp) = &u.order_whatsapp {
        if !wpp.is_empty() {
            println!("    📱 WhatsApp: {}", ui::format_phone(wpp));
        }
    }

    println!();
}

fn print_unit_details(u: &Unit, default_neighborhood: Option<&str>) {
    println!("\n{}", format!("📍 {} [{}]", u.name, u.id).red().bold());
    println!("    {}", u.formatted_address().italic());

    // Description
    if let Some(desc) = &u.description {
        if !desc.is_empty() {
            println!("\n    {}", "Sobre:".yellow());
            for line in desc.lines() {
                println!("    {}", line.truecolor(180, 180, 180));
            }
        }
    }

    // Contact
    println!("\n    {}", "Contato:".yellow());
    if let Some(phone) = &u.phone_number {
        if phone != "null" && !phone.is_empty() {
            println!("      Telefone:  {}", ui::format_phone(phone));
        }
    }
    if let Some(wpp) = &u.order_whatsapp {
        if !wpp.is_empty() {
            println!("      WhatsApp:  {}", ui::format_phone(wpp));
        }
    }
    if let Some(ig) = &u.instagram {
        if !ig.is_empty() {
            println!("      Instagram: @{}", ig);
        }
    }

    // Minimum order (prep time moved to modalities)
    if let Some(min_val) = u.minimum_order_value {
        if min_val > 0.0 {
            println!("      Pedido mín: {}", format!("R$ {:.2}", min_val).green());
        }
    }

    // Modalities (with prep time on Delivery)
    if let Some(flags) = &u.flags {
        let mut modos: Vec<String> = Vec::new();
        if flags.work_with_delivery {
            match u.preparation_time {
                Some(t) => modos.push(format!("Delivery (~{}min)", t)),
                None => modos.push("Delivery".to_string()),
            }
        }
        if flags.work_with_pick_up_store {
            modos.push("Retirada".to_string());
        }
        if flags.work_with_onsite {
            modos.push("No local".to_string());
        }
        if !modos.is_empty() {
            println!("      Modalidades: {}", modos.join(", ").green());
        }
    }

    // Business hours
    if let Some(bh) = &u.business_hours {
        println!("\n    {}", "Horário de funcionamento:".yellow());
        ui::print_day("      Dom", &bh.sunday);
        ui::print_day("      Seg", &bh.monday);
        ui::print_day("      Ter", &bh.tuesday);
        ui::print_day("      Qua", &bh.wednesday);
        ui::print_day("      Qui", &bh.thursday);
        ui::print_day("      Sex", &bh.friday);
        ui::print_day("      Sáb", &bh.saturday);
    }

    // Payment methods
    let active_payments: Vec<&str> = u
        .payment_methods
        .iter()
        .filter(|p| p.active.unwrap_or(false))
        .filter_map(|p| p.name.as_deref())
        .collect();
    if !active_payments.is_empty() {
        println!("\n    {}", "Formas de pagamento:".yellow());
        for name in &active_payments {
            println!("      • {}", name);
        }
    }

    // Delivery neighborhoods with default highlight
    if !u.delivery_only_for_neighborhoods.is_empty() {
        println!(
            "\n    {} ({} bairros)",
            "Bairros atendidos:".yellow(),
            u.delivery_only_for_neighborhoods.len()
        );

        let mut found_default = false;
        let names: Vec<(String, bool)> = u
            .delivery_only_for_neighborhoods
            .iter()
            .map(|n| {
                if let Some(dn) = default_neighborhood {
                    if n.name.to_lowercase() == dn.to_lowercase() {
                        found_default = true;
                        return (format!("  {:<25}", format!("{} ★", n.name)), true);
                    }
                }
                (format!("  {:<25}", n.name), false)
            })
            .collect();

        for chunk in names.chunks(3) {
            let has_highlight = chunk.iter().any(|(_, h)| *h);
            let line: String = chunk.iter().map(|(s, _)| s.as_str()).collect();
            if has_highlight {
                println!("      {}", line.yellow().bold());
            } else {
                println!("      {}", line.truecolor(180, 180, 180));
            }
        }

        if let Some(dn) = default_neighborhood {
            if !found_default {
                println!(
                    "\n    {} O bairro padrão '{}' não está na lista de bairros atendidos por esta unidade.",
                    "⚠".yellow(),
                    dn.bold()
                );
            }
        }
    }

    println!();
}

fn handle_set_default_unit(
    unit_id: u32,
    units: &[Unit],
    user_config: &Option<UserConfig>,
    opts: &AppOptions,
) {
    let unit = match units.iter().find(|u| u.id == unit_id) {
        Some(u) => u,
        None => {
            println!("{}", "Unidade não encontrada.".red());
            return;
        }
    };

    let mut cfg = match user_config.clone() {
        Some(c) => c,
        None => {
            println!(
                "Perfil não encontrado. Use {} primeiro.",
                "drpizza perfil --edit".cyan()
            );
            return;
        }
    };

    let idx = match cfg.endereco_padrao {
        Some(i) if i < cfg.addresses.len() => i,
        _ => {
            println!(
                "Nenhum endereço padrão definido. Use {} para adicionar.",
                "drpizza enderecos".cyan()
            );
            return;
        }
    };

    let addr_label = cfg.addresses[idx].label.clone();
    let neighborhood = cfg.addresses[idx].neighborhood.clone();

    if !unit_serves_neighborhood(unit, &neighborhood) {
        println!(
            "{} A unidade {} não atende o bairro '{}'.",
            "⚠".yellow(),
            unit.name.bold(),
            neighborhood.bold()
        );
    }

    let confirm = ui::read_input(&format!(
        "Definir {} como padrão para {}? (S/N): ",
        unit.name.bold(),
        addr_label.bold()
    ));
    if confirm.to_uppercase() != "S" {
        println!("Operação cancelada.");
        return;
    }

    cfg.addresses[idx].unidade_padrao = Some(unit_id);
    config::save_user_config(&cfg, opts);
    println!(
        "{} definida como unidade padrão para {}!",
        unit.name.green().bold(),
        addr_label.green().bold()
    );
}

fn handle_remove_default_unit(user_config: &Option<UserConfig>, opts: &AppOptions) {
    let mut cfg = match user_config.clone() {
        Some(c) => c,
        None => {
            println!(
                "Perfil não encontrado. Use {} primeiro.",
                "drpizza perfil --edit".cyan()
            );
            return;
        }
    };

    let idx = match cfg.endereco_padrao {
        Some(i) if i < cfg.addresses.len() => i,
        _ => {
            println!(
                "Nenhum endereço padrão definido. Use {} para adicionar.",
                "drpizza enderecos".cyan()
            );
            return;
        }
    };

    if cfg.addresses[idx].unidade_padrao.is_none() {
        println!("Nenhuma unidade padrão definida para este endereço.");
        return;
    }

    let addr_label = cfg.addresses[idx].label.clone();
    cfg.addresses[idx].unidade_padrao = None;
    config::save_user_config(&cfg, opts);
    println!(
        "Unidade padrão removida do endereço {}.",
        addr_label.green().bold()
    );
}
