// src/data.rs
use crate::models::{MenuCategory, Unit, LoyaltyReward};
use std::fs; // Import filesystem module

pub fn get_units() -> Vec<Unit> {
    vec![
        Unit { id: 1, name: "Dr. Pizza - Malvinas".to_string(), address: "R. Artur Corrêa de Brito, 205a".to_string() },
        Unit { id: 2, name: "Dr. Pizza - Cruzeiro".to_string(), address: "R. Aprígio Pereira Nepomuceno, 32".to_string() },
        Unit { id: 3, name: "Dr. Pizza - Alto Branco".to_string(), address: "Av. Manoel Tavares, 210".to_string() },
    ]
}

pub fn get_menu_data() -> Vec<MenuCategory> {
    let file_path = "menu.json";

    // Tries to read the file from the project root
    let data = fs::read_to_string(file_path).unwrap_or_else(|_| {
        eprintln!("❌ Erro Crítico: O arquivo '{}' não foi encontrado na raiz do projeto.", file_path);
        eprintln!("Certifique-se de criar o arquivo com o JSON do cardápio.");
        String::from("[]") // Return empty JSON array on error to prevent immediate crash, or use panic!()
    });

    serde_json::from_str(&data).expect("Erro ao parsear o JSON do menu em menu.json")
}

pub fn get_loyalty_rewards() -> Vec<LoyaltyReward> {
    // You can also move rewards to a file later (e.g., rewards.json)
    // For now, keeping your existing logic or the placeholder:
    let data = r#"
    [{"id":96459,"name":"Guaraná 1L","kind":"item","points_per_currency_unit":65,"active":true},{"id":70316,"name":"Desconto R$ 3,00","kind":"discount","points_quantity_required":300,"active":true}]
    "#;

    serde_json::from_str(data).expect("Erro ao parsear recompensas")
}