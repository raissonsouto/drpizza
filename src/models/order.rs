use crate::models::PaymentBrand;
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
    pub pix_qr_image: Option<String>,
    pub pix_qr_copy_paste: Option<String>,
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
    pub final_value: String,
    pub delivery_fee: f64,
    pub delivery_man_fee: Option<f64>,
    pub additional_fee: Option<f64>,
    pub estimated_time: u32,
    pub custom_fields_data: String,
    pub company_id: u32,
    pub confirmation: bool,
    pub order_type: String,
    pub payment_values_attributes: Vec<PaymentValuePayload>,
    pub scheduled_date: Option<String>,
    pub scheduled_period: Option<String>,
    pub earned_points: u32,
    pub sales_channel: String,
    pub customer_origin: Option<String>,
    pub diswpp_message_id: Option<String>,
    pub invoice_document: Option<String>,
    pub client_id: u64,
    pub client: OrderClientPayload,
    pub delivery_address: OrderAddressPayload,
    pub benefits: Vec<serde_json::Value>,
    pub order_items: Vec<OrderItemPayload>,
}

#[derive(Debug, Serialize)]
pub struct OrderClientPayload {
    pub name: String,
    pub ddi: u32,
    pub telephone: String,
}

#[derive(Debug, Serialize)]
pub struct OrderAddressPayload {
    pub street: String,
    pub neighborhood: String,
    pub address_complement: String,
    pub house_number: String,
    pub city: String,
    pub state: String,
    pub landmark: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub zip_code: String,
}

#[derive(Debug, Serialize)]
pub struct OrderItemPayload {
    pub item_id: u32,
    pub kind: String,
    pub name: String,
    pub custom_code: Option<String>,
    pub category_id: u32,
    pub category_name: String,
    pub quantity: u32,
    pub observation: String,
    pub unit_price: f64,
    pub price: f64,
    pub price_without_discounts: f64,
    pub print_area_id: Option<u32>,
    pub second_print_area_id: Option<u32>,
    pub order_subitems_attributes: Vec<OrderSubItemPayload>,
}

#[derive(Debug, Serialize)]
pub struct OrderSubItemPayload {
    pub subitem_id: u32,
    pub quantity: u32,
    pub price: f64,
    pub total_price: f64,
    pub name: String,
    pub custom_code: Option<String>,
    pub add_on_id: u32,
    pub add_on_name: String,
}

#[derive(Debug, Serialize)]
pub struct PaymentValuePayload {
    pub id: Option<u64>,
    pub name: String,
    pub fixed_fee: Option<f64>,
    pub percentual_fee: Option<f64>,
    pub available_on_menu: bool,
    pub available_for: Vec<String>,
    pub available_order_timings: Vec<String>,
    pub allow_on_customer_first_order: bool,
    pub online_payment_provider: Option<String>,
    pub kind: String,
    pub brands: Vec<PaymentBrandPayload>,
    pub payment_method_id: Option<u64>,
    pub payment_method: String,
    pub payment_method_brand_id: Option<u64>,
    pub payment_fee: Option<f64>,
    pub total: f64,
}

#[derive(Debug, Serialize, Clone)]
pub struct PaymentBrandPayload {
    pub id: Option<u64>,
    pub name: Option<String>,
    pub kind: Option<String>,
    pub image_key: Option<String>,
    pub system_default: bool,
}

impl From<&PaymentBrand> for PaymentBrandPayload {
    fn from(value: &PaymentBrand) -> Self {
        Self {
            id: value.id,
            name: value.name.clone(),
            kind: value.kind.clone(),
            image_key: value.image_key.clone(),
            system_default: value.system_default.unwrap_or(false),
        }
    }
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct OrderResponse {
    pub id: u64,
    pub uid: String,
    pub order_number: u64,
    pub status: String,
}
