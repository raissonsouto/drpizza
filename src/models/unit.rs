use serde::{Deserialize, Serialize};

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
