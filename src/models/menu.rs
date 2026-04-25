use serde::{Deserialize, Serialize};

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
    pub custom_code: Option<String>,
    pub description: Option<String>,
    pub price: f64,
    pub promotional_price: Option<f64>,
    pub promotional_price_active: bool,
    pub kind: String,
    pub print_area_id: Option<u32>,
    pub second_print_area_id: Option<u32>,
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
    pub custom_code: Option<String>,
    pub price: f64,
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

#[derive(Clone, Debug)]
pub struct CartItem {
    pub item_id: u32,
    pub name: String,
    pub custom_code: Option<String>,
    pub print_area_id: Option<u32>,
    pub second_print_area_id: Option<u32>,
    pub category_id: u32,
    pub category_name: String,
    pub flavors: Vec<String>,
    pub flavor_ids: Vec<u32>,
    pub crust: String,
    pub crust_id: Option<u32>,
    pub extras: Vec<SelectedSubItem>,
    pub price: f64,
    pub price_without_discounts: f64,
}

#[derive(Clone, Debug)]
pub struct MenuSelection {
    pub item: MenuItem,
    pub category_id: u32,
    pub category_name: String,
    pub flavors: Vec<SubItem>,
    pub crust: Option<SubItem>,
    pub extras: Vec<SelectedSubItem>,
}

#[derive(Clone, Debug)]
pub struct SelectedSubItem {
    pub id: u32,
    pub name: String,
    pub price: f64,
    pub quantity: u32,
    pub add_on_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LoyaltyReward {
    pub id: u32,
    pub name: String,
    pub kind: String,
    pub fixed_discount_value: Option<f64>,
    pub points_quantity_required: Option<u32>,
    pub points_per_currency_unit: Option<f64>,
    pub active: bool,
}
