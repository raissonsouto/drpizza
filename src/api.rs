use crate::models::{
    CepResponse, GroupResponse, MenuCategory, OrderAddressPayload, OrderClientPayload, OrderDetail,
    OrderItemPayload, OrderPayload, OrderResponse, OrderSubItemPayload, PaymentBrandPayload,
    PaymentValuePayload, PendingOrder, Unit,
};
use hmac::{Hmac, Mac};
use rand::Rng;
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::Sha256;
use std::error::Error;

type HmacSha256 = Hmac<Sha256>;
const ORDER_TRACE_SECRET: &str = "jEYqVsGg83ZRJgw97yrK";

macro_rules! dev_log {
    ($($arg:tt)*) => {
        #[cfg(feature = "dev")]
        eprintln!("[DEV] {}", format!($($arg)*));
    };
}

#[cfg(feature = "dev")]
fn shell_single_quote(s: &str) -> String {
    s.replace('\'', "'\"'\"'")
}

#[cfg(feature = "dev")]
fn shell_dollar_quote_bytes(bytes: &[u8]) -> String {
    let mut out = String::from("$'");
    for &b in bytes {
        match b {
            b'\\' => out.push_str("\\\\"),
            b'\'' => out.push_str("\\'"),
            b'\n' => out.push_str("\\n"),
            b'\r' => out.push_str("\\r"),
            b'\t' => out.push_str("\\t"),
            0x20..=0x7e => out.push(b as char),
            _ => out.push_str(&format!("\\x{:02x}", b)),
        }
    }
    out.push('\'');
    out
}

#[cfg(feature = "dev")]
fn dev_log_curl<T: Serialize>(
    method: &str,
    url: &str,
    headers: &[(&str, String)],
    body: Option<&T>,
) {
    let mut parts = vec![format!("curl '{}' -X {}", shell_single_quote(url), method)];
    for (k, v) in headers {
        parts.push(format!(
            "-H '{}: {}'",
            shell_single_quote(k),
            shell_single_quote(v)
        ));
    }
    if let Some(payload) = body {
        if let Ok(json) = serde_json::to_string(payload) {
            parts.push(format!("--data-raw '{}'", shell_single_quote(&json)));
        }
    }
    dev_log!("cURL: {}", parts.join(" \\\n  "));
}

#[cfg(feature = "dev")]
fn dev_log_curl_bytes(method: &str, url: &str, headers: &[(&str, String)], body: Option<&[u8]>) {
    let mut parts = vec![format!("curl '{}' -X {}", shell_single_quote(url), method)];
    for (k, v) in headers {
        parts.push(format!(
            "-H '{}: {}'",
            shell_single_quote(k),
            shell_single_quote(v)
        ));
    }
    if let Some(payload) = body {
        parts.push(format!("--data-raw {}", shell_dollar_quote_bytes(payload)));
    }
    dev_log!("cURL: {}", parts.join(" \\\n  "));
}

#[cfg(not(feature = "dev"))]
fn dev_log_curl<T: Serialize>(
    _method: &str,
    _url: &str,
    _headers: &[(&str, String)],
    _body: Option<&T>,
) {
}

#[cfg(not(feature = "dev"))]
fn dev_log_curl_bytes(
    _method: &str,
    _url: &str,
    _headers: &[(&str, String)],
    _body: Option<&[u8]>,
) {
}

#[cfg(feature = "dev")]
fn dev_log_response(status: reqwest::StatusCode, headers: &HeaderMap, body_text: &str) {
    let request_id = headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-");
    dev_log!("Response Status: {}", status);
    dev_log!("Response x-request-id: {}", request_id);
    dev_log!("Response Body: {}", body_text);
}

#[cfg(not(feature = "dev"))]
fn dev_log_response(_status: reqwest::StatusCode, _headers: &HeaderMap, _body_text: &str) {}

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
    (0..10)
        .map(|_| chars[rng.gen_range(0..chars.len())])
        .collect()
}

fn append_json_string(out: &mut Vec<u8>, text: &str, _legacy_latin1: bool) {
    out.push(b'"');
    for ch in text.chars() {
        match ch {
            '"' => out.extend_from_slice(br#"\""#),
            '\\' => out.extend_from_slice(br#"\\"#),
            '\u{08}' => out.extend_from_slice(br#"\b"#),
            '\u{0c}' => out.extend_from_slice(br#"\f"#),
            '\n' => out.extend_from_slice(br#"\n"#),
            '\r' => out.extend_from_slice(br#"\r"#),
            '\t' => out.extend_from_slice(br#"\t"#),
            c if (c as u32) < 0x20 => {
                out.extend_from_slice(format!("\\u{:04x}", c as u32).as_bytes());
            }
            c => {
                let mut buf = [0u8; 4];
                out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
            }
        }
    }
    out.push(b'"');
}

fn append_json_key(out: &mut Vec<u8>, key: &str) {
    append_json_string(out, key, false);
    out.push(b':');
}

fn append_separator(out: &mut Vec<u8>, first: &mut bool) {
    if *first {
        *first = false;
    } else {
        out.push(b',');
    }
}

fn append_json_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(value.to_string().as_bytes());
}

fn append_json_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(value.to_string().as_bytes());
}

fn append_json_bool(out: &mut Vec<u8>, value: bool) {
    out.extend_from_slice(if value { b"true" } else { b"false" });
}

fn append_json_f64(out: &mut Vec<u8>, value: f64) {
    if let Some(number) = serde_json::Number::from_f64(value) {
        let raw = number.to_string();
        if let Some(int_like) = raw.strip_suffix(".0") {
            out.extend_from_slice(int_like.as_bytes());
        } else {
            out.extend_from_slice(raw.as_bytes());
        }
    } else {
        out.extend_from_slice(b"null");
    }
}

fn append_json_opt_f64(out: &mut Vec<u8>, value: Option<f64>) {
    match value {
        Some(value) => append_json_f64(out, value),
        None => out.extend_from_slice(b"null"),
    }
}

fn append_json_opt_u64(out: &mut Vec<u8>, value: Option<u64>) {
    match value {
        Some(value) => append_json_u64(out, value),
        None => out.extend_from_slice(b"null"),
    }
}

fn append_json_opt_string(out: &mut Vec<u8>, value: Option<&str>, legacy_latin1: bool) {
    match value {
        Some(value) => append_json_string(out, value, legacy_latin1),
        None => out.extend_from_slice(b"null"),
    }
}

fn append_string_array(out: &mut Vec<u8>, values: &[String], legacy_latin1: bool) {
    out.push(b'[');
    for (idx, value) in values.iter().enumerate() {
        if idx > 0 {
            out.push(b',');
        }
        append_json_string(out, value, legacy_latin1);
    }
    out.push(b']');
}

fn append_value_array(out: &mut Vec<u8>, values: &[Value]) -> Result<(), Box<dyn Error>> {
    out.push(b'[');
    for (idx, value) in values.iter().enumerate() {
        if idx > 0 {
            out.push(b',');
        }
        out.extend_from_slice(&serde_json::to_vec(value)?);
    }
    out.push(b']');
    Ok(())
}

fn serialize_payment_brand_payload(out: &mut Vec<u8>, brand: &PaymentBrandPayload) {
    out.push(b'{');
    let mut first = true;

    append_separator(out, &mut first);
    append_json_key(out, "id");
    append_json_opt_u64(out, brand.id);

    append_separator(out, &mut first);
    append_json_key(out, "name");
    append_json_opt_string(out, brand.name.as_deref(), true);

    append_separator(out, &mut first);
    append_json_key(out, "kind");
    append_json_opt_string(out, brand.kind.as_deref(), true);

    append_separator(out, &mut first);
    append_json_key(out, "system_default");
    append_json_bool(out, brand.system_default);

    out.push(b'}');
}

fn serialize_payment_value_payload(out: &mut Vec<u8>, payment: &PaymentValuePayload) {
    out.push(b'{');
    let mut first = true;

    append_separator(out, &mut first);
    append_json_key(out, "id");
    append_json_opt_u64(out, payment.id);

    append_separator(out, &mut first);
    append_json_key(out, "name");
    append_json_string(out, &payment.name, true);

    append_separator(out, &mut first);
    append_json_key(out, "fixed_fee");
    append_json_opt_f64(out, payment.fixed_fee);

    append_separator(out, &mut first);
    append_json_key(out, "percentual_fee");
    append_json_opt_f64(out, payment.percentual_fee);

    append_separator(out, &mut first);
    append_json_key(out, "available_on_menu");
    append_json_bool(out, payment.available_on_menu);

    append_separator(out, &mut first);
    append_json_key(out, "available_for");
    append_string_array(out, &payment.available_for, false);

    append_separator(out, &mut first);
    append_json_key(out, "available_order_timings");
    append_string_array(out, &payment.available_order_timings, false);

    append_separator(out, &mut first);
    append_json_key(out, "allow_on_customer_first_order");
    append_json_bool(out, payment.allow_on_customer_first_order);

    append_separator(out, &mut first);
    append_json_key(out, "online_payment_provider");
    append_json_opt_string(out, payment.online_payment_provider.as_deref(), true);

    append_separator(out, &mut first);
    append_json_key(out, "kind");
    append_json_string(out, &payment.kind, false);

    append_separator(out, &mut first);
    append_json_key(out, "brands");
    out.push(b'[');
    for (idx, brand) in payment.brands.iter().enumerate() {
        if idx > 0 {
            out.push(b',');
        }
        serialize_payment_brand_payload(out, brand);
    }
    out.push(b']');

    append_separator(out, &mut first);
    append_json_key(out, "payment_method_id");
    append_json_opt_u64(out, payment.payment_method_id);

    append_separator(out, &mut first);
    append_json_key(out, "payment_method");
    append_json_string(out, &payment.payment_method, false);

    append_separator(out, &mut first);
    append_json_key(out, "payment_method_brand_id");
    append_json_opt_u64(out, payment.payment_method_brand_id);

    append_separator(out, &mut first);
    append_json_key(out, "payment_fee");
    append_json_opt_f64(out, payment.payment_fee);

    append_separator(out, &mut first);
    append_json_key(out, "total");
    append_json_f64(out, payment.total);

    out.push(b'}');
}

fn serialize_order_client_payload(out: &mut Vec<u8>, client: &OrderClientPayload) {
    out.push(b'{');
    let mut first = true;

    append_separator(out, &mut first);
    append_json_key(out, "name");
    append_json_string(out, &client.name, true);

    append_separator(out, &mut first);
    append_json_key(out, "ddi");
    append_json_u32(out, client.ddi);

    append_separator(out, &mut first);
    append_json_key(out, "telephone");
    append_json_string(out, &client.telephone, false);

    out.push(b'}');
}

fn serialize_order_address_payload(out: &mut Vec<u8>, address: &OrderAddressPayload) {
    out.push(b'{');
    let mut first = true;

    append_separator(out, &mut first);
    append_json_key(out, "street");
    append_json_string(out, &address.street, true);

    append_separator(out, &mut first);
    append_json_key(out, "neighborhood");
    append_json_string(out, &address.neighborhood, true);

    append_separator(out, &mut first);
    append_json_key(out, "address_complement");
    append_json_string(out, &address.address_complement, true);

    append_separator(out, &mut first);
    append_json_key(out, "house_number");
    append_json_string(out, &address.house_number, false);

    append_separator(out, &mut first);
    append_json_key(out, "city");
    append_json_string(out, &address.city, true);

    append_separator(out, &mut first);
    append_json_key(out, "state");
    append_json_string(out, &address.state, false);

    append_separator(out, &mut first);
    append_json_key(out, "landmark");
    append_json_string(out, &address.landmark, false);

    append_separator(out, &mut first);
    append_json_key(out, "latitude");
    append_json_opt_f64(out, address.latitude);

    append_separator(out, &mut first);
    append_json_key(out, "longitude");
    append_json_opt_f64(out, address.longitude);

    append_separator(out, &mut first);
    append_json_key(out, "zip_code");
    append_json_string(out, &address.zip_code, false);

    out.push(b'}');
}

fn serialize_order_subitem_payload(out: &mut Vec<u8>, subitem: &OrderSubItemPayload) {
    out.push(b'{');
    let mut first = true;

    append_separator(out, &mut first);
    append_json_key(out, "subitem_id");
    append_json_u32(out, subitem.subitem_id);

    append_separator(out, &mut first);
    append_json_key(out, "quantity");
    append_json_u32(out, subitem.quantity);

    append_separator(out, &mut first);
    append_json_key(out, "price");
    append_json_f64(out, subitem.price);

    append_separator(out, &mut first);
    append_json_key(out, "total_price");
    append_json_f64(out, subitem.total_price);

    append_separator(out, &mut first);
    append_json_key(out, "name");
    append_json_string(out, &subitem.name, true);

    append_separator(out, &mut first);
    append_json_key(out, "custom_code");
    append_json_opt_string(out, subitem.custom_code.as_deref(), false);

    append_separator(out, &mut first);
    append_json_key(out, "add_on_id");
    append_json_u32(out, subitem.add_on_id);

    append_separator(out, &mut first);
    append_json_key(out, "add_on_name");
    append_json_string(out, &subitem.add_on_name, true);

    out.push(b'}');
}

fn serialize_order_item_payload(out: &mut Vec<u8>, item: &OrderItemPayload) {
    out.push(b'{');
    let mut first = true;

    append_separator(out, &mut first);
    append_json_key(out, "item_id");
    append_json_u32(out, item.item_id);

    append_separator(out, &mut first);
    append_json_key(out, "kind");
    append_json_string(out, &item.kind, false);

    append_separator(out, &mut first);
    append_json_key(out, "name");
    append_json_string(out, &item.name, true);

    append_separator(out, &mut first);
    append_json_key(out, "custom_code");
    append_json_opt_string(out, item.custom_code.as_deref(), false);

    append_separator(out, &mut first);
    append_json_key(out, "quantity");
    append_json_u32(out, item.quantity);

    append_separator(out, &mut first);
    append_json_key(out, "observation");
    append_json_string(out, &item.observation, true);

    append_separator(out, &mut first);
    append_json_key(out, "unit_price");
    append_json_f64(out, item.unit_price);

    append_separator(out, &mut first);
    append_json_key(out, "price");
    append_json_f64(out, item.price);

    append_separator(out, &mut first);
    append_json_key(out, "price_without_discounts");
    append_json_f64(out, item.price_without_discounts);

    append_separator(out, &mut first);
    append_json_key(out, "print_area_id");
    match item.print_area_id {
        Some(value) => append_json_u32(out, value),
        None => out.extend_from_slice(b"null"),
    }

    append_separator(out, &mut first);
    append_json_key(out, "second_print_area_id");
    match item.second_print_area_id {
        Some(value) => append_json_u32(out, value),
        None => out.extend_from_slice(b"null"),
    }

    append_separator(out, &mut first);
    append_json_key(out, "category_id");
    append_json_u32(out, item.category_id);

    append_separator(out, &mut first);
    append_json_key(out, "category_name");
    append_json_string(out, &item.category_name, true);

    append_separator(out, &mut first);
    append_json_key(out, "order_subitems_attributes");
    out.push(b'[');
    for (idx, subitem) in item.order_subitems_attributes.iter().enumerate() {
        if idx > 0 {
            out.push(b',');
        }
        serialize_order_subitem_payload(out, subitem);
    }
    out.push(b']');

    out.push(b'}');
}

fn serialize_order_payload_legacy(payload: &OrderPayload) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut out = Vec::with_capacity(4096);
    let mut first = true;
    out.push(b'{');

    append_separator(&mut out, &mut first);
    append_json_key(&mut out, "final_value");
    append_json_string(&mut out, &payload.final_value, false);

    append_separator(&mut out, &mut first);
    append_json_key(&mut out, "delivery_fee");
    append_json_f64(&mut out, payload.delivery_fee);

    append_separator(&mut out, &mut first);
    append_json_key(&mut out, "delivery_man_fee");
    append_json_opt_f64(&mut out, payload.delivery_man_fee);

    append_separator(&mut out, &mut first);
    append_json_key(&mut out, "additional_fee");
    append_json_opt_f64(&mut out, payload.additional_fee);

    append_separator(&mut out, &mut first);
    append_json_key(&mut out, "estimated_time");
    append_json_u32(&mut out, payload.estimated_time);

    append_separator(&mut out, &mut first);
    append_json_key(&mut out, "custom_fields_data");
    append_json_string(&mut out, &payload.custom_fields_data, false);

    append_separator(&mut out, &mut first);
    append_json_key(&mut out, "company_id");
    append_json_u32(&mut out, payload.company_id);

    append_separator(&mut out, &mut first);
    append_json_key(&mut out, "confirmation");
    append_json_bool(&mut out, payload.confirmation);

    append_separator(&mut out, &mut first);
    append_json_key(&mut out, "order_type");
    append_json_string(&mut out, &payload.order_type, false);

    append_separator(&mut out, &mut first);
    append_json_key(&mut out, "payment_values_attributes");
    out.push(b'[');
    for (idx, payment) in payload.payment_values_attributes.iter().enumerate() {
        if idx > 0 {
            out.push(b',');
        }
        serialize_payment_value_payload(&mut out, payment);
    }
    out.push(b']');

    append_separator(&mut out, &mut first);
    append_json_key(&mut out, "scheduled_date");
    append_json_opt_string(&mut out, payload.scheduled_date.as_deref(), false);

    append_separator(&mut out, &mut first);
    append_json_key(&mut out, "scheduled_period");
    append_json_opt_string(&mut out, payload.scheduled_period.as_deref(), false);

    append_separator(&mut out, &mut first);
    append_json_key(&mut out, "earned_points");
    append_json_u32(&mut out, payload.earned_points);

    append_separator(&mut out, &mut first);
    append_json_key(&mut out, "sales_channel");
    append_json_string(&mut out, &payload.sales_channel, false);

    append_separator(&mut out, &mut first);
    append_json_key(&mut out, "customer_origin");
    append_json_opt_string(&mut out, payload.customer_origin.as_deref(), true);

    append_separator(&mut out, &mut first);
    append_json_key(&mut out, "diswpp_message_id");
    append_json_opt_string(&mut out, payload.diswpp_message_id.as_deref(), false);

    append_separator(&mut out, &mut first);
    append_json_key(&mut out, "invoice_document");
    append_json_opt_string(&mut out, payload.invoice_document.as_deref(), false);

    append_separator(&mut out, &mut first);
    append_json_key(&mut out, "client_id");
    append_json_u64(&mut out, payload.client_id);

    append_separator(&mut out, &mut first);
    append_json_key(&mut out, "client");
    serialize_order_client_payload(&mut out, &payload.client);

    append_separator(&mut out, &mut first);
    append_json_key(&mut out, "delivery_address");
    serialize_order_address_payload(&mut out, &payload.delivery_address);

    append_separator(&mut out, &mut first);
    append_json_key(&mut out, "benefits");
    append_value_array(&mut out, &payload.benefits)?;

    append_separator(&mut out, &mut first);
    append_json_key(&mut out, "order_items");
    out.push(b'[');
    for (idx, item) in payload.order_items.iter().enumerate() {
        if idx > 0 {
            out.push(b',');
        }
        serialize_order_item_payload(&mut out, item);
    }
    out.push(b']');

    out.push(b'}');
    Ok(out)
}

fn compute_order_trace_id(body: &[u8]) -> Result<String, Box<dyn Error>> {
    let mut mac = HmacSha256::new_from_slice(ORDER_TRACE_SECRET.as_bytes())?;
    mac.update(body);
    let digest = mac.finalize().into_bytes();
    let hex = digest
        .iter()
        .map(|byte| format!("{:02x}", byte))
        .collect::<String>();
    Ok(hex[7..23].to_string())
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
    dev_log_curl::<serde_json::Value>(
        "GET",
        url,
        &[
            ("company-id", ctx.company_id.to_string()),
            ("company", ctx.company_slug.clone()),
            ("sessionid", ctx.session_id.clone()),
        ],
        None,
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
    let response_headers = res.headers().clone();
    let body_text = res.text().await?;

    dev_log_response(status, &response_headers, &body_text);

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

#[derive(Debug, Serialize)]
struct ClientSessionLoginPayload {
    auth: ClientSessionAuthPayload,
}

#[derive(Debug, Serialize)]
struct ClientSessionAuthPayload {
    client_id: u64,
    password: String,
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
    dev_log_curl(
        "POST",
        url,
        &[
            ("Accept", "application/json, text/plain, */*".to_string()),
            ("Content-Type", "application/json;charset=utf-8".to_string()),
            ("company-id", ctx.company_id.to_string()),
            ("company", ctx.company_slug.clone()),
            ("sessionid", ctx.session_id.clone()),
        ],
        Some(&payload),
    );

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
            return find_client_by_phone(ctx, phone).await;
        }

        return Err(format!("Erro na API de clientes: {}", error_text).into());
    }

    let json_response: serde_json::Value = res.json().await?;
    dev_log!("Response: {}", json_response);

    let client_id = extract_client_id(&json_response)?;
    let token = extract_token(&json_response);
    Ok(ClientResult { client_id, token })
}

pub async fn find_client_by_phone(
    ctx: &ApiContext,
    phone: &str,
) -> Result<ClientResult, Box<dyn Error>> {
    let digits: String = phone.chars().filter(|c| c.is_ascii_digit()).collect();
    let url = format!(
        "https://integracao.cardapioweb.com/api/menu/company/clients?ddi=55&telephone={}",
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
        .or_else(|| json["access_token"].as_str())
        .or_else(|| json["auth_token"].as_str())
        .or_else(|| json["authorization"].as_str())
        .or_else(|| json["client"]["token"].as_str())
        .or_else(|| json["data"]["token"].as_str())
        .or_else(|| json["data"]["access_token"].as_str())
        .or_else(|| json["data"]["authorization"].as_str())
        .map(|s| s.to_string())
}

pub async fn login_client_session(
    ctx: &ApiContext,
    client_id: u64,
    password: &str,
) -> Result<String, Box<dyn Error>> {
    let url = "https://integracao.cardapioweb.com/api/menu/authentication/client_session/login";
    dev_log!(
        "POST {} client_id={} company-id={} company={}",
        url,
        client_id,
        ctx.company_id,
        ctx.company_slug
    );

    let payload = ClientSessionLoginPayload {
        auth: ClientSessionAuthPayload {
            client_id,
            password: password.to_string(),
        },
    };
    dev_log_curl(
        "POST",
        url,
        &[
            ("Accept", "application/json, text/plain, */*".to_string()),
            ("Content-Type", "application/json;charset=utf-8".to_string()),
            ("company-id", ctx.company_id.to_string()),
            ("company", ctx.company_slug.clone()),
            ("sessionid", ctx.session_id.clone()),
        ],
        Some(&payload),
    );

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
    let response_headers = res.headers().clone();
    let body_text = res.text().await?;
    dev_log_response(status, &response_headers, &body_text);

    if !status.is_success() {
        return Err(format!("Erro no login da sessão do cliente: {}", body_text).into());
    }

    let json_response: serde_json::Value = serde_json::from_str(&body_text)?;
    extract_token(&json_response)
        .ok_or_else(|| "Login realizado, mas não foi possível extrair token de autenticação".into())
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
pub struct DeliveryQuote {
    pub value: f64,
    pub estimated_time: Option<u32>,
}

pub async fn calculate_delivery_tax(
    ctx: &ApiContext,
    street: &str,
    number: &str,
    neighborhood: &str,
    city: &str,
    state: &str,
    zip_code: &str,
) -> Result<DeliveryQuote, Box<dyn Error>> {
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
    dev_log_curl(
        "POST",
        url,
        &[
            ("Accept", "application/json, text/plain, */*".to_string()),
            ("Content-Type", "application/json;charset=utf-8".to_string()),
            ("company-id", ctx.company_id.to_string()),
            ("company", ctx.company_slug.clone()),
            ("sessionid", ctx.session_id.clone()),
        ],
        Some(&payload),
    );

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
    let estimated_time = json_response["estimated_time"]
        .as_u64()
        .map(|value| value as u32);

    Ok(DeliveryQuote {
        value: tax,
        estimated_time,
    })
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

// --- Submit Order ---

pub async fn submit_order(
    ctx: &ApiContext,
    payload: &OrderPayload,
    _auth_token: Option<&str>,
) -> Result<OrderResponse, Box<dyn Error>> {
    let url = "https://integracao.cardapioweb.com/api/menu/company/orders/new_version";
    let encoded_body = serialize_order_payload_legacy(payload)?;
    let trace_id = compute_order_trace_id(&encoded_body)?;
    dev_log!("POST {}", url);
    dev_log!(
        "Headers: company-id={}, company={}, sessionid={}, trace-id={}",
        ctx.company_id,
        ctx.company_slug,
        ctx.session_id,
        trace_id
    );
    let debug_headers = vec![
        ("Accept", "application/json, text/plain, */*".to_string()),
        ("Accept-Language", "en-US,en;q=0.9".to_string()),
        ("Content-Type", "application/json;charset=utf-8".to_string()),
        ("company-id", ctx.company_id.to_string()),
        ("company", ctx.company_slug.clone()),
        ("sessionid", ctx.session_id.clone()),
        ("Trace-Id", trace_id.clone()),
        ("Origin", "https://app.cardapioweb.com".to_string()),
        ("Referer", "https://app.cardapioweb.com/".to_string()),
        (
            "User-Agent",
            "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:148.0) Gecko/20100101 Firefox/148.0"
                .to_string(),
        ),
        ("Sec-Fetch-Dest", "empty".to_string()),
        ("Sec-Fetch-Mode", "cors".to_string()),
        ("Sec-Fetch-Site", "same-site".to_string()),
    ];

    dev_log_curl_bytes("POST", url, &debug_headers, Some(&encoded_body));

    let client = reqwest::Client::new();
    let req = client
        .post(url)
        .header("Accept", "application/json, text/plain, */*")
        .header("Accept-Language", "en-US,en;q=0.9")
        .header("Content-Type", "application/json;charset=utf-8")
        .header("company-id", ctx.company_id.to_string())
        .header("company", &ctx.company_slug)
        .header("sessionid", &ctx.session_id)
        .header("Trace-Id", trace_id)
        .header("Origin", "https://app.cardapioweb.com")
        .header("Referer", "https://app.cardapioweb.com/")
        .header(
            "User-Agent",
            "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:148.0) Gecko/20100101 Firefox/148.0",
        )
        .header("Sec-Fetch-Dest", "empty")
        .header("Sec-Fetch-Mode", "cors")
        .header("Sec-Fetch-Site", "same-site")
        .body(encoded_body);

    let res = req.send().await?;

    let status = res.status();
    let response_headers = res.headers().clone();
    let request_id = res
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let body_text = res.text().await?;
    dev_log_response(status, &response_headers, &body_text);

    if !status.is_success() {
        let req_suffix = request_id
            .as_deref()
            .map(|id| format!(" [x-request-id: {}]", id))
            .unwrap_or_default();
        return Err(format!(
            "Erro ao enviar pedido: {}{} - {}",
            status, req_suffix, body_text
        )
        .into());
    }

    let order: OrderResponse = serde_json::from_str(&body_text)?;
    dev_log!(
        "Pedido criado: id={}, uid={}, number={}, status={}",
        order.id,
        order.uid,
        order.order_number,
        order.status
    );

    Ok(order)
}

#[cfg(test)]
mod tests {
    use super::{compute_order_trace_id, serialize_order_payload_legacy};
    use crate::models::{
        OrderAddressPayload, OrderClientPayload, OrderItemPayload, OrderPayload,
        PaymentBrandPayload, PaymentValuePayload,
    };

    fn contains_subslice(haystack: &[u8], needle: &[u8]) -> bool {
        haystack.windows(needle.len()).any(|w| w == needle)
    }

    #[test]
    fn serializes_order_payload_as_utf8() {
        let payload = OrderPayload {
            final_value: "47.90".to_string(),
            delivery_fee: 8.0,
            delivery_man_fee: None,
            additional_fee: None,
            estimated_time: 80,
            custom_fields_data: "[]".to_string(),
            company_id: 7842,
            confirmation: false,
            order_type: "delivery".to_string(),
            payment_values_attributes: vec![PaymentValuePayload {
                id: Some(44164),
                name: "Pix automático".to_string(),
                fixed_fee: None,
                percentual_fee: None,
                available_on_menu: true,
                available_for: vec![],
                available_order_timings: vec![],
                allow_on_customer_first_order: true,
                online_payment_provider: None,
                kind: "pix_auto".to_string(),
                brands: vec![PaymentBrandPayload {
                    id: Some(49068),
                    name: None,
                    kind: None,
                    image_key: None,
                    system_default: true,
                }],
                payment_method_id: Some(44164),
                payment_method: "pix_auto".to_string(),
                payment_method_brand_id: Some(49068),
                payment_fee: None,
                total: 47.9,
            }],
            scheduled_date: None,
            scheduled_period: None,
            earned_points: 39,
            sales_channel: "catalog".to_string(),
            customer_origin: None,
            diswpp_message_id: None,
            invoice_document: None,
            client_id: 60319762,
            client: OrderClientPayload {
                name: "Raisson Souto".to_string(),
                ddi: 55,
                telephone: "83998498006".to_string(),
            },
            delivery_address: OrderAddressPayload {
                street: "Rua Pedro Feitosa Neves".to_string(),
                neighborhood: "Bela Vista".to_string(),
                address_complement: "506A".to_string(),
                house_number: "465".to_string(),
                city: "Campina Grande".to_string(),
                state: "PB".to_string(),
                landmark: "residencial bellagio por trás do ct do campinense".to_string(),
                latitude: None,
                longitude: None,
                zip_code: "58428757".to_string(),
            },
            benefits: vec![],
            order_items: vec![OrderItemPayload {
                item_id: 849749,
                kind: "regular_item".to_string(),
                name: "Coca-Cola Zero 1L".to_string(),
                custom_code: Some("12065344".to_string()),
                category_id: 92051,
                category_name: "BEBIDAS".to_string(),
                quantity: 1,
                observation: String::new(),
                unit_price: 11.9,
                price: 11.9,
                price_without_discounts: 11.9,
                print_area_id: Some(16062),
                second_print_area_id: None,
                order_subitems_attributes: vec![],
            }],
        };

        let bytes = serialize_order_payload_legacy(&payload).expect("serialize order payload");
        assert!(contains_subslice(
            &bytes,
            br#"{"final_value":"47.90","delivery_fee":8,"delivery_man_fee":null,"additional_fee":null,"estimated_time":80"#,
        ));
        assert!(contains_subslice(&bytes, b"Pix autom\xc3\xa1tico"));
        assert!(!contains_subslice(&bytes, b"Pix autom\xe1tico"));
        assert!(contains_subslice(
            &bytes,
            b"residencial bellagio por tr\xc3\xa1s do ct do campinense"
        ));
    }

    #[test]
    fn computes_trace_id_like_browser() {
        let body = br#"{"a":1}"#;
        let trace_id = compute_order_trace_id(body).expect("compute trace id");
        assert_eq!(trace_id, "817d70cb11ee2367");
    }
}
