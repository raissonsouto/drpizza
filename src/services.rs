use std::io::{self, Write};
use std::{thread, time};
use crate::models::{CartItem, MenuItem, LoyaltyReward};
use crate::data;
use colored::*;

// --- Helper Functions ---

fn clear_screen() {
    print!("\x1B[2J\x1B[1;1H");
    io::stdout().flush().unwrap();
}

fn read_input(prompt: &str) -> String {
    print!("{}", prompt);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

fn read_int(prompt: &str) -> Option<u32> {
    let input = read_input(prompt);
    input.parse::<u32>().ok()
}

/// Helper to assign icons based on category name
fn get_category_icon(name: &str) -> &str {
    let n = name.to_lowercase();
    if n.contains("pizza") { "🍕" }
    else if n.contains("bebida") || n.contains("refrigerante") || n.contains("suco") { "🥤" }
    else if n.contains("combo") { "📦" }
    else if n.contains("bread") || n.contains("pão") { "🥖" }
    else { "🍽️" }
}

// --- Logic ---

pub fn list_menu() {
    let menu_data = data::get_menu_data();

    if menu_data.is_empty() {
        println!("{}", "⚠️  O cardápio está vazio ou o arquivo 'menu.json' não foi encontrado.".red());
        return;
    }

    println!("\n{}", "📜 --- CARDÁPIO DR. PIZZA ---".on_red().white().bold());

    for category in menu_data {
        let icon = get_category_icon(&category.name);
        // Header Category
        println!("\n{} {}", icon, category.name.to_uppercase().yellow().bold());
        println!("{}", "-".repeat(60).bright_black());

        for item in category.items {
            let price = item.get_current_price();
            let price_str = format!("R$ {:.2}", price).green().bold();
            
            // Format: Name ...................... Price
            println!("   • {:.<50} {}", item.name.white(), price_str);

            if let Some(desc) = &item.description {
                if !desc.is_empty() {
                    println!("     {}", desc.italic().truecolor(150, 150, 150));
                }
            }
        }
    }
    println!();
    println!("Use o comando {} para iniciar o pedido.", "drpizza order".cyan());
}

pub fn list_rewards() {
    let rewards = data::get_loyalty_rewards();
    println!("\n{}", "🏆 --- PROGRAMA DE FIDELIDADE ---".yellow().bold());
    
    for r in rewards {
        if r.active {
            let kind_display = match r.kind.as_str() {
                "item" => "Item Grátis".cyan(),
                "discount" => "Desconto".green(),
                _ => "Benefício".white()
            };
            println!("   ★ {} [{}]", r.name.bold(), kind_display);
        }
    }
    println!();
}

pub fn list_units() {
    println!("\n{}", "📍 --- UNIDADES DISPONÍVEIS ---".red().bold());
    for u in data::get_units() {
        println!("[{}] {}\n    └─ {}", u.id.to_string().cyan(), u.name.bold(), u.address.italic());
    }
    println!();
}

pub fn start_order_flow(pre_selected_unit_id: Option<u32>) {
    let units = data::get_units();
    let menu_data = data::get_menu_data();

    if menu_data.is_empty() {
        println!("{}", "⚠️  Aviso: O cardápio está vazio. Verifique 'menu.json'.".red());
    }

    clear_screen();
    println!("{}", "Bem-vindo ao Dr. Pizza CLI! 🍕".on_red().white().bold());

    // 1. Select Unit
    let selected_unit = if let Some(uid) = pre_selected_unit_id {
        units.iter().find(|u| u.id == uid).cloned().expect("Unidade não encontrada")
    } else {
        loop {
            list_units();
            if let Some(choice) = read_int("Digite o número da unidade: ") {
                if let Some(u) = units.iter().find(|u| u.id == choice) {
                    break u.clone(); 
                }
            }
            println!("{}", "❌ Opção inválida.".red());
        }
    };

    println!("{} {}", "✅ Conectado:".green(), selected_unit.name);
    thread::sleep(time::Duration::from_secs(1));

    // 2. Order Loop Setup
    let mut cart: Vec<CartItem> = Vec::new();
    let mut total_price = 0.0;
    let mut ordering = true;

    // Flatten items logic: We create a flat list to map IDs (1, 2, 3...) to items,
    // but we will DISPLAY them using the categories structure.
    let mut all_items_flat: Vec<MenuItem> = Vec::new();
    for cat in &menu_data {
        for item in &cat.items {
            all_items_flat.push(item.clone());
        }
    }

    while ordering {
        clear_screen();
        println!("{}", format!("🏠 Loja: {}", selected_unit.name).on_blue().white());
        println!("{}", "--------------------------------".bright_black());

        // --- VISUAL DISPLAY LOOP ---
        let mut display_counter = 1;
        for category in &menu_data {
            println!("\n{} {}", get_category_icon(&category.name), category.name.yellow().bold());
            
            for item in &category.items {
                let id_display = format!("[{:02}]", display_counter).cyan().bold();
                let price_display = format!("R$ {:.2}", item.get_current_price()).green();
                
                // Ex: [01] Pizza Calabresa ........... R$ 50.00
                println!(" {} {:.<45} {}", id_display, item.name, price_display);
                
                display_counter += 1;
            }
        }
        // ---------------------------

        let selected_item = loop {
            println!("\n{}", "--------------------------------".bright_black());
            if let Some(idx) = read_int("👉 Escolha o número do item (0 p/ finalizar): ") {
                if idx == 0 {
                    ordering = false;
                    break None; // Break internal loop
                }
                if idx > 0 && idx <= all_items_flat.len() as u32 {
                    break Some(all_items_flat[(idx - 1) as usize].clone());
                }
            }
            println!("{}", "❌ Opção inválida.".red());
        };

        // Handle selection
        if let Some(item) = selected_item {
            // Add to Cart
            cart.push(CartItem {
                name: item.name.clone(),
                crust: "Tradicional".to_string(), // You can add crust selection logic here later
                price: item.get_current_price(),
            });
            total_price += item.get_current_price();

            println!("\n✅ {} adicionado!", item.name.green());
            println!("🛒 Subtotal: {}", format!("R$ {:.2}", total_price).green().bold());
            
            thread::sleep(time::Duration::from_secs(1));
            
            let cont = read_input("\nPedir mais algo? (S/N): ");
            if cont.trim().to_uppercase() != "S" {
                ordering = false;
            }
        } else {
            // User typed 0 to finish
            ordering = false; 
        }
    }

    // 3. Checkout Summary & Loyalty
    clear_screen();
    println!("{}", "📝 RESUMO DO PEDIDO".on_white().black().bold());
    println!("{}", "-------------------".bright_black());
    
    if cart.is_empty() {
        println!("Carrinho vazio.");
        return;
    }

    for item in &cart {
        println!("• {:.<40} {}", item.name, format!("R$ {:.2}", item.price).green());
    }
    println!("{}", "-------------------".bright_black());
    println!("💰 TOTAL: {}", format!("R$ {:.2}", total_price).green().bold().on_black());
    
    let points_earned = total_price as u32;
    println!("\n{}", format!("🎁 Fidelidade: Você ganhou {} pontos!", points_earned).magenta());
    
    println!("\n👀 Ver catálogo de prêmios? (S/N)");
    let ver_premios = read_input("> ");
    if ver_premios.to_uppercase() == "S" {
        list_rewards();
    }
    
    read_input("\nPressione ENTER para confirmar o pedido...");
    println!("\n{}", "🚀 Enviando pedido... Aguarde a pizza! 🍕".green().bold());
}