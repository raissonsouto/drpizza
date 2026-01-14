// src/models.rs
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Unit {
    pub id: u32,
    pub name: String,
    pub address: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MenuCategory {
    pub id: u32,
    pub name: String,
    pub description: Option<String>,
    pub items: Vec<MenuItem>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MenuItem {
    pub id: u32,
    pub name: String,
    pub description: Option<String>,
    pub price: f64,
    pub promotional_price: Option<f64>,
    pub promotional_price_active: bool,
    pub kind: String, // "regular_item" or "combo"
    pub add_ons: Vec<AddOnGroup>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AddOnGroup {
    pub id: u32,
    pub name: String,
    pub subitems: Vec<SubItem>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SubItem {
    pub id: u32,
    pub name: String,
    pub price: f64,
}

#[derive(Clone, Debug)]
pub struct CartItem {
    pub name: String,
    pub crust: String,
    pub price: f64,
}

// --- NOVO: Struct para o Programa de Fidelidade ---
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LoyaltyReward {
    pub id: u32,
    pub name: String,
    pub kind: String, // "discount", "item", "free_delivery"
    pub fixed_discount_value: Option<f64>,
    pub points_quantity_required: Option<u32>,
    pub points_per_currency_unit: Option<f64>, // Em alguns casos no JSON isso parece ser o custo
    pub active: bool,
}

impl MenuItem {
    pub fn get_current_price(&self) -> f64 {
        if self.promotional_price_active {
            self.promotional_price.unwrap_or(self.price)
        } else {
            self.price
        }
    }
}