use crate::config::{self, AppOptions};
use crate::models::{AddOnGroup, MenuCategory, MenuItem, MenuSelection, SubItem};
use crate::ui;
use crate::units;
use colored::*;
use std::cmp;

pub async fn list_menu(opts: &AppOptions, no_pagination: bool) {
    let (_unit, ctx) = units::select_unit_and_context(opts).await;
    let menu_data = config::get_menu_data(&ctx, opts).await;

    if menu_data.is_empty() {
        println!(
            "{}",
            "O cardápio está vazio ou não foi possível carregar.".red()
        );
        return;
    }

    if no_pagination {
        print_full_menu(&menu_data);
    } else {
        browse_menu(&menu_data);
    }
}

fn print_full_menu(menu_data: &[MenuCategory]) {
    println!(
        "\n{}",
        "📜 --- CARDÁPIO DR. PIZZA ---".on_red().white().bold()
    );

    let mut counter = 1;
    for category in menu_data {
        let icon = ui::get_category_icon(&category.name);
        println!(
            "\n{} {}",
            icon,
            category.name.to_uppercase().yellow().bold()
        );
        println!("{}", "-".repeat(60).bright_black());

        for item in &category.items {
            let num = format!("[{:02}]", counter).cyan().bold();
            let price = item.get_current_price();
            let price_str = format!("R$ {:.2}", price).green().bold();
            println!(" {} {:.<45} {}", num, item.name.white(), price_str);

            if let Some(desc) = &item.description {
                if !desc.is_empty() {
                    print_wrapped_desc(desc);
                }
            }

            print_item_addons(item);
            counter += 1;
        }
    }
    println!();
}

fn browse_menu(menu_data: &[MenuCategory]) {
    loop {
        println!(
            "\n{}",
            "📜 --- CARDÁPIO DR. PIZZA ---".on_red().white().bold()
        );

        for (i, cat) in menu_data.iter().enumerate() {
            let icon = ui::get_category_icon(&cat.name);
            let count = cat.items.len();
            println!(
                "  [{}] {} {} {}",
                (i + 1).to_string().cyan().bold(),
                icon,
                cat.name.to_uppercase().yellow(),
                format!("({} itens)", count).bright_black()
            );
        }

        println!("\n[S] Sair");

        let choice = ui::read_input("\n> ");
        if choice.to_uppercase() == "S" {
            break;
        }

        if let Ok(idx) = choice.parse::<usize>() {
            if idx >= 1 && idx <= menu_data.len() {
                show_category_items(&menu_data[idx - 1]);
                continue;
            }
        }
        println!("{}", "Opção inválida.".red());
    }
}

/// Browse menu with item selection (used by `drpizza pedir`).
/// Returns `Some(selection)` when user picks an item, `None` to finish.
pub fn browse_menu_select(menu_data: &[MenuCategory]) -> Option<MenuSelection> {
    loop {
        println!(
            "\n{}",
            "📜 --- CARDÁPIO DR. PIZZA ---".on_red().white().bold()
        );

        for (i, cat) in menu_data.iter().enumerate() {
            let icon = ui::get_category_icon(&cat.name);
            let count = cat.items.len();
            println!(
                "  [{}] {} {} {}",
                (i + 1).to_string().cyan().bold(),
                icon,
                cat.name.to_uppercase().yellow(),
                format!("({} itens)", count).bright_black()
            );
        }

        println!("\n[S] Sair");

        let choice = ui::read_input("\n> ");
        if choice.to_uppercase() == "S" {
            return None;
        }

        if let Ok(idx) = choice.parse::<usize>() {
            if idx >= 1 && idx <= menu_data.len() {
                if let Some(selection) = select_category_item(&menu_data[idx - 1]) {
                    return Some(selection);
                }
                continue;
            }
        }
        println!("{}", "Opção inválida.".red());
    }
}

fn show_category_items(category: &MenuCategory) {
    let icon = ui::get_category_icon(&category.name);
    println!(
        "\n{} {}",
        icon,
        category.name.to_uppercase().yellow().bold()
    );
    println!("{}", "-".repeat(60).bright_black());

    for (i, item) in category.items.iter().enumerate() {
        let num = format!("[{:02}]", i + 1).cyan().bold();
        let price = item.get_current_price();
        let price_str = format!("R$ {:.2}", price).green().bold();
        println!(" {} {:.<45} {}", num, item.name.white(), price_str);

        if let Some(desc) = &item.description {
            if !desc.is_empty() {
                print_wrapped_desc(desc);
            }
        }

        print_item_addons(item);
    }
    println!();
}

fn select_category_item(category: &MenuCategory) -> Option<MenuSelection> {
    let icon = ui::get_category_icon(&category.name);
    println!(
        "\n{} {}",
        icon,
        category.name.to_uppercase().yellow().bold()
    );
    println!("{}", "-".repeat(60).bright_black());

    for (i, item) in category.items.iter().enumerate() {
        let num = format!("[{:02}]", i + 1).cyan().bold();
        let price = item.get_current_price();
        let price_str = format!("R$ {:.2}", price).green().bold();
        println!(" {} {:.<45} {}", num, item.name.white(), price_str);

        if let Some(desc) = &item.description {
            if !desc.is_empty() {
                print_wrapped_desc(desc);
            }
        }

        print_item_addons(item);
    }

    loop {
        if let Some(idx) = ui::read_int("\nEscolha o item (0 p/ voltar): ") {
            if idx == 0 {
                return None;
            }
            if idx >= 1 && idx <= category.items.len() as u32 {
                let item = category.items[(idx - 1) as usize].clone();
                let flavors = select_flavors(&item);
                let crust = select_crust(&item);
                return Some(MenuSelection {
                    item,
                    flavors,
                    crust,
                });
            }
        }
        println!("{}", "Opção inválida.".red());
    }
}

// --- Add-on display ---

fn print_item_addons(item: &MenuItem) {
    if item.add_ons.is_empty() {
        return;
    }

    for addon in &item.add_ons {
        let kind = classify_addon(&addon.name);
        match kind {
            AddonKind::Flavors => {
                let names: Vec<&str> = addon.subitems.iter().map(|s| s.name.as_str()).collect();
                if !names.is_empty() {
                    println!(
                        "      {} {}",
                        "Sabores:".truecolor(180, 140, 80),
                        preview_list(&names, 10).truecolor(150, 150, 150)
                    );
                }
            }
            AddonKind::Crusts => {
                let crusts: Vec<String> = addon
                    .subitems
                    .iter()
                    .map(|s| {
                        if s.price > 0.0 {
                            format!("{} (+R${:.2})", s.name, s.price)
                        } else {
                            s.name.clone()
                        }
                    })
                    .collect();
                if !crusts.is_empty() {
                    println!(
                        "      {} {}",
                        "Bordas:".truecolor(180, 140, 80),
                        preview_list(&crusts, 6).truecolor(150, 150, 150)
                    );
                }
            }
            AddonKind::Other => {}
        }
    }
}

// --- Flavor / Crust selection ---

fn select_flavors(item: &MenuItem) -> Vec<SubItem> {
    let addon = match find_addon(item, AddonKind::Flavors) {
        Some(a) => a,
        None => return Vec::new(),
    };

    let max = parse_max_flavors(&addon.name);

    println!(
        "\n{} {}",
        "Sabores disponíveis".yellow().bold(),
        format!("(escolha até {})", max).bright_black()
    );
    for (i, sub) in addon.subitems.iter().enumerate() {
        if sub.price > 0.0 {
            println!(
                "  [{}] {} {}",
                (i + 1).to_string().cyan(),
                sub.name,
                format!("R$ {:.2}", sub.price).green()
            );
        } else {
            println!("  [{}] {}", (i + 1).to_string().cyan(), sub.name);
        }
    }

    let mut selected: Vec<SubItem> = Vec::new();
    for n in 1..=max {
        let prompt = if max == 1 {
            "Sabor: ".to_string()
        } else {
            format!("Sabor {} de {}: ", n, max)
        };
        loop {
            if let Some(idx) = ui::read_int(&prompt) {
                if idx >= 1 && idx <= addon.subitems.len() as u32 {
                    selected.push(addon.subitems[(idx - 1) as usize].clone());
                    break;
                }
            }
            println!("{}", "Opção inválida.".red());
        }
    }

    selected
}

fn select_crust(item: &MenuItem) -> Option<SubItem> {
    let addon = match find_addon(item, AddonKind::Crusts) {
        Some(a) => a,
        None => return None,
    };

    println!("\n{}", "Bordas disponíveis".yellow().bold());
    for (i, sub) in addon.subitems.iter().enumerate() {
        if sub.price > 0.0 {
            println!(
                "  [{}] {} {}",
                (i + 1).to_string().cyan(),
                sub.name,
                format!("+R$ {:.2}", sub.price).green()
            );
        } else {
            println!("  [{}] {}", (i + 1).to_string().cyan(), sub.name);
        }
    }

    loop {
        if let Some(idx) = ui::read_int("Borda: ") {
            if idx >= 1 && idx <= addon.subitems.len() as u32 {
                return Some(addon.subitems[(idx - 1) as usize].clone());
            }
        }
        println!("{}", "Opção inválida.".red());
    }
}

// --- Helpers ---

#[derive(PartialEq)]
enum AddonKind {
    Flavors,
    Crusts,
    Other,
}

fn classify_addon(name: &str) -> AddonKind {
    let lower = name.to_lowercase();
    if lower.contains("sabor") {
        AddonKind::Flavors
    } else if lower.contains("borda") {
        AddonKind::Crusts
    } else {
        AddonKind::Other
    }
}

fn find_addon(item: &MenuItem, kind: AddonKind) -> Option<&AddOnGroup> {
    item.add_ons
        .iter()
        .find(|a| classify_addon(&a.name) == kind)
}

fn parse_max_flavors(addon_name: &str) -> usize {
    // "Sabores Pizza - 2 Sabores" -> 2
    for word in addon_name.split_whitespace() {
        if let Ok(n) = word.parse::<usize>() {
            if n > 0 && n <= 10 {
                return n;
            }
        }
    }
    1
}

fn print_wrapped_desc(text: &str) {
    for line in wrap_text(text, 90) {
        println!("      {}", line.italic().truecolor(150, 150, 150));
    }
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let mut out = Vec::new();
    for raw_line in text.lines() {
        let words: Vec<&str> = raw_line.split_whitespace().collect();
        if words.is_empty() {
            out.push(String::new());
            continue;
        }
        let mut current = String::new();
        for word in words {
            if current.is_empty() {
                current.push_str(word);
                continue;
            }
            if current.len() + 1 + word.len() <= width {
                current.push(' ');
                current.push_str(word);
            } else {
                out.push(current);
                current = word.to_string();
            }
        }
        if !current.is_empty() {
            out.push(current);
        }
    }
    out
}

fn preview_list<T: AsRef<str>>(items: &[T], max_items: usize) -> String {
    let take_n = cmp::min(max_items, items.len());
    let mut shown: Vec<String> = items[..take_n]
        .iter()
        .map(|s| s.as_ref().to_string())
        .collect();
    if items.len() > max_items {
        shown.push(format!("... +{} opções", items.len() - max_items));
    }
    shown.join(", ")
}
