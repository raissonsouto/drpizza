use serde::{Deserialize, Serialize};

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
    #[serde(default)]
    pub price: Option<f64>,
    #[serde(default)]
    pub unit_price: Option<f64>,
    pub order_subitems: Vec<OrderSubItem>,
}

impl OrderItem {
    pub fn display_price(&self) -> f64 {
        self.unit_price.or(self.price).unwrap_or(0.0)
    }
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

#[derive(Debug, Serialize)]
pub struct OrderPayload {
    pub order: OrderData,
}

#[derive(Debug, Serialize)]
pub struct OrderData {
    pub order_type: String,
    pub client_id: u64,
    pub observation: String,
    pub delivery_address: OrderAddressPayload,
    pub order_items: Vec<OrderItemPayload>,
    pub payment_values: Vec<PaymentValuePayload>,
}

#[derive(Debug, Serialize)]
pub struct OrderAddressPayload {
    pub street: String,
    pub house_number: String,
    pub neighborhood: String,
    pub city: String,
    pub state: String,
    pub zip_code: String,
    pub landmark: String,
    pub address_complement: String,
}

#[derive(Debug, Serialize)]
pub struct OrderItemPayload {
    pub item_id: u32,
    pub quantity: u32,
    pub price: f64,
    pub order_subitems: Vec<OrderSubItemPayload>,
}

#[derive(Debug, Serialize)]
pub struct OrderSubItemPayload {
    pub subitem_id: u32,
    pub price: f64,
    pub quantity: u32,
}

#[derive(Debug, Serialize)]
pub struct PaymentValuePayload {
    pub payment_method: String,
    pub total: f64,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct OrderResponse {
    pub id: u64,
    pub uid: String,
    pub order_number: u64,
    pub status: String,
}
