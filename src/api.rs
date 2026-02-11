use crate::models::{CepResponse, GroupResponse, MenuCategory, OrderDetail, PendingOrder, Unit};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::error::Error;

macro_rules! dev_log {
    ($($arg:tt)*) => {
        #[cfg(feature = "dev")]
        eprintln!("[DEV] {}", format!($($arg)*));
    };
}

// --- ApiContext ---

pub struct ApiContext {
    pub company_id: u32,
    pub company_slug: String,
    pub session_id: String,
}

impl ApiContext {
    pub fn from_unit(unit: &Unit) -> Self {
        let slug = unit.url_name.clone().unwrap_or_default();
        let session_id = generate_session_id();
        dev_log!(
            "ApiContext: company_id={}, slug={}, session={}",
            unit.id,
            slug,
            session_id
        );
        ApiContext {
            company_id: unit.id,
            company_slug: slug,
            session_id,
        }
    }
}

fn generate_session_id() -> String {
    let mut rng = rand::thread_rng();
    let chars: Vec<char> = "abcdefghijklmnopqrstuvwxyz0123456789".chars().collect();
    let part1: String = (0..8)
        .map(|_| chars[rng.gen_range(0..chars.len())])
        .collect();
    let part2: String = (0..5)
        .map(|_| chars[rng.gen_range(0..chars.len())])
        .collect();
    format!("{}.{}", part1, part2)
}

// --- Units ---

pub async fn fetch_units() -> Result<Vec<Unit>, Box<dyn Error>> {
    let url = "https://integracao.cardapioweb.com/api/menu/users/drpizza";
    dev_log!("GET {}", url);

    let response = reqwest::get(url).await?;
    dev_log!("Status: {}", response.status());

    let menu_data: GroupResponse = response.json().await?;
    dev_log!("Unidades recebidas: {}", menu_data.units.len());
    Ok(menu_data.units)
}

// --- Menu ---

pub async fn fetch_menu(ctx: &ApiContext) -> Result<Vec<MenuCategory>, Box<dyn Error>> {
    let url = "https://integracao.cardapioweb.com/api/menu/company/categories?only_available_for=delivery&origin=catalogo";
    dev_log!("GET {}", url);
    dev_log!(
        "Headers: company-id={}, company={}, sessionid={}",
        ctx.company_id,
        ctx.company_slug,
        ctx.session_id
    );

    let client = reqwest::Client::new();
    let res = client
        .get(url)
        .header("company-id", ctx.company_id.to_string())
        .header("company", &ctx.company_slug)
        .header("sessionid", &ctx.session_id)
        .send()
        .await?;

    let status = res.status();
    let body_text = res.text().await?;

    dev_log!("Status: {}", status);
    dev_log!(
        "Body ({} bytes): {}",
        body_text.len(),
        &body_text[..body_text.len().min(500)]
    );

    if !status.is_success() {
        return Err(format!("Erro na API do cardápio: {}", status).into());
    }

    let categories: Vec<MenuCategory> = serde_json::from_str(&body_text)?;
    dev_log!("Categorias recebidas: {}", categories.len());

    Ok(categories)
}

// --- Client Registration ---

pub struct ClientResult {
    pub client_id: u64,
    pub token: Option<String>,
}

pub async fn register_client(
    ctx: &ApiContext,
    name: &str,
    phone: &str,
) -> Result<ClientResult, Box<dyn Error>> {
    let url = "https://integracao.cardapioweb.com/api/menu/company/clients";
    let digits: String = phone.chars().filter(|c| c.is_ascii_digit()).collect();
    dev_log!("POST {} name={} phone={}", url, name, digits);

    let payload = serde_json::json!({
        "name": name,
        "telephone": digits,
    });

    let client = reqwest::Client::new();
    let res = client
        .post(url)
        .header("Accept", "application/json, text/plain, */*")
        .header("Content-Type", "application/json;charset=utf-8")
        .header("company-id", ctx.company_id.to_string())
        .header("company", &ctx.company_slug)
        .header("sessionid", &ctx.session_id)
        .json(&payload)
        .send()
        .await?;

    let status = res.status();
    dev_log!("Status: {}", status);

    if !status.is_success() {
        let error_text = res.text().await?;
        dev_log!("Erro body: {}", error_text);

        // Phone already registered — look up existing client
        if error_text.contains("cadastrado") {
            return lookup_client_by_phone(ctx, phone).await;
        }

        return Err(format!("Erro na API de clientes: {}", error_text).into());
    }

    let json_response: serde_json::Value = res.json().await?;
    dev_log!("Response: {}", json_response);

    let client_id = extract_client_id(&json_response)?;
    let token = extract_token(&json_response);
    Ok(ClientResult { client_id, token })
}

async fn lookup_client_by_phone(
    ctx: &ApiContext,
    phone: &str,
) -> Result<ClientResult, Box<dyn Error>> {
    let digits: String = phone.chars().filter(|c| c.is_ascii_digit()).collect();
    let url = format!(
        "https://integracao.cardapioweb.com/api/menu/company/clients?telephone={}",
        digits
    );
    dev_log!("GET {} (lookup by phone)", url);

    let client = reqwest::Client::new();
    let res = client
        .get(&url)
        .header("Accept", "application/json, text/plain, */*")
        .header("company-id", ctx.company_id.to_string())
        .header("company", &ctx.company_slug)
        .header("sessionid", &ctx.session_id)
        .send()
        .await?;

    let status = res.status();
    dev_log!("Status: {}", status);

    if !status.is_success() {
        let error_text = res.text().await?;
        dev_log!("Erro body: {}", error_text);
        return Err(format!("Erro ao buscar cliente: {}", error_text).into());
    }

    let json_response: serde_json::Value = res.json().await?;
    dev_log!("Response: {}", json_response);

    let client_id = extract_client_id(&json_response)?;
    let token = extract_token(&json_response);
    Ok(ClientResult { client_id, token })
}

fn extract_client_id(json: &serde_json::Value) -> Result<u64, Box<dyn Error>> {
    json["client"]["id"]
        .as_u64()
        .or_else(|| json["client"]["id"].as_str().and_then(|s| s.parse().ok()))
        .or_else(|| json["id"].as_u64())
        .or_else(|| json["id"].as_str().and_then(|s| s.parse().ok()))
        .or_else(|| {
            json.as_array()
                .and_then(|arr| arr.first())
                .and_then(|c| c["id"].as_u64())
        })
        .ok_or_else(|| "Não foi possível extrair o ID do cliente".into())
}

fn extract_token(json: &serde_json::Value) -> Option<String> {
    json["token"]
        .as_str()
        .or_else(|| json["auth_token"].as_str())
        .or_else(|| json["authorization"].as_str())
        .or_else(|| json["client"]["token"].as_str())
        .map(|s| s.to_string())
}

// --- CEP Lookup ---

pub async fn lookup_cep(cep: &str) -> Result<CepResponse, Box<dyn Error>> {
    let clean_cep = cep.replace('-', "");
    let url = format!("https://viacep.com.br/ws/{}/json/", clean_cep);
    dev_log!("GET {}", url);

    let res = reqwest::get(&url).await?;
    dev_log!("Status: {}", res.status());

    let data: CepResponse = res.json().await?;
    Ok(data)
}

// --- Delivery Tax ---

#[derive(Debug, Serialize)]
struct CalculateTaxPayload {
    calculate_tax: AddressData,
}

#[derive(Debug, Serialize)]
struct AddressData {
    latitude: Option<f64>,
    longitude: Option<f64>,
    street: String,
    house_number: String,
    neighborhood: String,
    city: String,
    state: String,
    zip_code: String,
    full_address: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct DeliveryResponse {
    pub value: f64,
}

pub async fn calculate_delivery_tax(
    ctx: &ApiContext,
    street: &str,
    number: &str,
    neighborhood: &str,
    city: &str,
    state: &str,
    zip_code: &str,
) -> Result<f64, Box<dyn Error>> {
    let url = "https://integracao.cardapioweb.com/api/menu/company/orders/calculate_tax";
    let full_address = format!(
        "{}, {}, {}, {}, {}",
        street, number, neighborhood, city, state
    );
    dev_log!("POST {}", url);
    dev_log!("Endereço: {}", full_address);

    let payload = CalculateTaxPayload {
        calculate_tax: AddressData {
            latitude: None,
            longitude: None,
            street: street.to_string(),
            house_number: number.to_string(),
            neighborhood: neighborhood.to_string(),
            city: city.to_string(),
            state: state.to_string(),
            zip_code: zip_code.to_string(),
            full_address,
        },
    };

    let client = reqwest::Client::new();
    let res = client
        .post(url)
        .header("Accept", "application/json, text/plain, */*")
        .header("Content-Type", "application/json;charset=utf-8")
        .header("company-id", ctx.company_id.to_string())
        .header("company", &ctx.company_slug)
        .header("sessionid", &ctx.session_id)
        .json(&payload)
        .send()
        .await?;

    let status = res.status();
    dev_log!("Status: {}", status);

    if !status.is_success() {
        let error_text = res.text().await?;
        dev_log!("Erro body: {}", error_text);
        return Err(format!("Erro na API: {}", error_text).into());
    }

    let json_response: serde_json::Value = res.json().await?;
    dev_log!("Response: {}", json_response);

    let tax = json_response["data"]["price"]
        .as_f64()
        .or_else(|| json_response["value"].as_f64())
        .or_else(|| {
            json_response["value"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
        })
        .ok_or("Não foi possível extrair o valor da taxa de entrega")?;

    Ok(tax)
}

// --- Pending Orders ---

pub async fn fetch_pending_orders(
    ctx: &ApiContext,
    client_id: u64,
) -> Result<Vec<PendingOrder>, Box<dyn Error>> {
    let url = format!(
        "https://integracao.cardapioweb.com/api/menu/company/client/{}/pending_orders",
        client_id
    );
    dev_log!("GET {}", url);

    let client = reqwest::Client::new();
    let res = client
        .get(&url)
        .header("accept", "application/json, text/plain, */*")
        .header("company-id", ctx.company_id.to_string())
        .header("company", &ctx.company_slug)
        .header("sessionid", &ctx.session_id)
        .send()
        .await?;

    let status = res.status();
    dev_log!("Status: {}", status);

    if !status.is_success() {
        let error_text = res.text().await?;
        dev_log!("Erro body: {}", error_text);
        return Err(format!("Erro na API: {}", error_text).into());
    }

    let orders: Vec<PendingOrder> = res.json().await?;
    dev_log!("Pedidos recebidos: {}", orders.len());
    Ok(orders)
}

// --- Closed Orders ---

pub async fn fetch_closed_orders(
    ctx: &ApiContext,
    client_id: u64,
    limit: u32,
    auth_token: Option<&str>,
) -> Result<Vec<PendingOrder>, Box<dyn Error>> {
    let url = format!(
        "https://integracao.cardapioweb.com/api/menu/company/client/{}/closed_orders?limit={}",
        client_id, limit
    );
    dev_log!("GET {}", url);

    let client = reqwest::Client::new();
    let mut req = client
        .get(&url)
        .header("accept", "application/json, text/plain, */*")
        .header("company-id", ctx.company_id.to_string())
        .header("company", &ctx.company_slug)
        .header("sessionid", &ctx.session_id);

    if let Some(token) = auth_token {
        req = req.header("authorization", token);
    }

    let res = req.send().await?;

    let status = res.status();
    dev_log!("Status: {}", status);

    if !status.is_success() {
        let error_text = res.text().await?;
        dev_log!("Erro body: {}", error_text);
        return Err(format!("Erro na API: {}", error_text).into());
    }

    let orders: Vec<PendingOrder> = res.json().await?;
    dev_log!("Pedidos fechados recebidos: {}", orders.len());
    Ok(orders)
}

// --- Order Detail ---

pub async fn fetch_order_detail(
    ctx: &ApiContext,
    order_uid: &str,
) -> Result<OrderDetail, Box<dyn Error>> {
    let url = format!(
        "https://integracao.cardapioweb.com/api/menu/company/orders/{}",
        order_uid
    );
    dev_log!("GET {}", url);

    let client = reqwest::Client::new();
    let res = client
        .get(&url)
        .header("accept", "application/json, text/plain, */*")
        .header("company-id", ctx.company_id.to_string())
        .header("company", &ctx.company_slug)
        .header("sessionid", &ctx.session_id)
        .send()
        .await?;

    let status = res.status();
    dev_log!("Status: {}", status);

    if !status.is_success() {
        let error_text = res.text().await?;
        dev_log!("Erro body: {}", error_text);
        return Err(format!("Erro na API: {}", error_text).into());
    }

    let detail: OrderDetail = res.json().await?;
    Ok(detail)
}
