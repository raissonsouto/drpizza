use serde::{Deserialize, Serialize};

// --- Unit / Group ---

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Unit {
    pub id: u32,
    pub uuid: String,
    pub name: String,
    pub description: Option<String>,

    pub phone_number: Option<String>,
    pub order_whatsapp: Option<String>,
    pub instagram: Option<String>,

    pub url_name: Option<String>,
    pub image: Option<String>,
    pub logo: Option<String>,
    pub thumbnail: Option<String>,

    pub city: Option<String>,
    pub street: Option<String>,
    pub state: Option<String>,
    pub address_number: Option<String>,
    pub address_complement: Option<String>,
    pub neighborhood: Option<String>,

    pub latitude: Option<String>,
    pub longitude: Option<String>,

    pub preparation_time: Option<u32>,
    pub minimum_order_value: Option<f64>,

    pub flags: Option<UnitFlags>,
    pub business_hours: Option<BusinessHours>,

    #[serde(default)]
    pub delivery_only_for_neighborhoods: Vec<DeliveryNeighborhood>,
    #[serde(default)]
    pub payment_methods: Vec<PaymentMethod>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeliveryNeighborhood {
    pub name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PaymentMethod {
    pub name: Option<String>,
    pub method: Option<String>,
    pub active: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UnitFlags {
    pub work_with_delivery: bool,
    pub work_with_pick_up_store: bool,
    pub work_with_onsite: bool,
    pub work_with_scheduled_order: bool,
    pub automatic_order_closing: bool,
    pub show_categories_first: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BusinessHours {
    pub sunday: Option<Vec<Vec<String>>>,
    pub monday: Option<Vec<Vec<String>>>,
    pub tuesday: Option<Vec<Vec<String>>>,
    pub wednesday: Option<Vec<Vec<String>>>,
    pub thursday: Option<Vec<Vec<String>>>,
    pub friday: Option<Vec<Vec<String>>>,
    pub saturday: Option<Vec<Vec<String>>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GroupResponse {
    pub id: u32,
    pub group_name: String,
    #[serde(rename = "companies")]
    pub units: Vec<Unit>,
}

impl Unit {
    pub fn formatted_address(&self) -> String {
        let street = self.street.as_deref().unwrap_or("");
        let number = self.address_number.as_deref().unwrap_or("");
        let neighbor = self.neighborhood.as_deref().unwrap_or("");
        let city = self.city.as_deref().unwrap_or("");
        let state = self.state.as_deref().unwrap_or("");
        format!(
            "{}, {} - {}, {} - {}",
            street, number, neighbor, city, state
        )
    }
}

// --- Menu ---

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
    pub kind: String,
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

impl MenuItem {
    pub fn get_current_price(&self) -> f64 {
        if self.promotional_price_active {
            self.promotional_price.unwrap_or(self.price)
        } else {
            self.price
        }
    }
}

// --- Cart ---

#[derive(Clone, Debug)]
pub struct CartItem {
    pub name: String,
    pub flavors: Vec<String>,
    pub crust: String,
    pub price: f64,
}

// --- Menu Selection ---

#[derive(Clone, Debug)]
pub struct MenuSelection {
    pub item: MenuItem,
    pub flavors: Vec<SubItem>,
    pub crust: Option<SubItem>,
}

// --- Loyalty ---

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

// --- CEP Lookup ---

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CepResponse {
    pub cep: String,
    pub logradouro: String,
    #[serde(default)]
    pub complemento: String,
    pub bairro: String,
    pub localidade: String,
    pub uf: String,
}

// --- Pending Orders ---

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PendingOrder {
    pub id: u64,
    pub order_number: u64,
    pub status: String,
    pub order_type: Option<String>,
    pub final_value: f64,
    pub uid: String,
    pub created_at: String,
    pub updated_at: Option<String>,
    pub status_changes: Vec<StatusChange>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StatusChange {
    pub id: u64,
    pub status: String,
    pub created_at: String,
    pub user_name: Option<String>,
}

// --- Order Detail ---

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrderDetail {
    pub id: u64,
    pub uid: String,
    pub order_number: u64,
    pub status: String,
    pub order_type: Option<String>,
    pub delivery_fee: Option<f64>,
    pub final_value: f64,
    pub earned_points: Option<u64>,
    pub observation: Option<String>,
    pub created_at: String,
    pub order_items: Vec<OrderItem>,
    pub delivery_address: Option<DeliveryAddress>,
    pub payment_values: Vec<PaymentValue>,
    pub status_changes: Vec<StatusChange>,
    pub client: Option<OrderClient>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrderItem {
    pub name: String,
    pub quantity: f64,
    #[serde(alias = "unit_price")]
    pub price: f64,
    pub order_subitems: Vec<OrderSubItem>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrderSubItem {
    pub name: String,
    pub price: f64,
    pub add_on_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PaymentValue {
    pub total: f64,
    pub payment_type: Option<String>,
    pub payment_method: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrderClient {
    pub id: u64,
    pub name: String,
    pub telephone: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeliveryAddress {
    pub street: Option<String>,
    pub house_number: Option<String>,
    pub neighborhood: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub zip_code: Option<String>,
    pub landmark: Option<String>,
    pub address_complement: Option<String>,
}

// --- Menu Cache ---

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MenuCache {
    pub fetched_at: String,
    pub company_slug: String,
    pub categories: Vec<MenuCategory>,
}

// --- User Config (~/.drpizza) ---

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct UserConfig {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub phone: String,
    pub client_id: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_token: Option<String>,
    #[serde(default)]
    pub addresses: Vec<SavedAddress>,
    #[serde(default)]
    pub endereco_padrao: Option<usize>,
    #[serde(default)]
    pub nao_perguntar_unidade: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SavedAddress {
    pub label: String,
    pub cep: String,
    pub street: String,
    pub number: String,
    pub complement: String,
    pub neighborhood: String,
    pub city: String,
    pub state: String,
    pub landmark: String,
    #[serde(default)]
    pub unidade_padrao: Option<u32>,
}
