use serde::{Deserialize, Serialize};

use super::menu::MenuCategory;

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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MenuCache {
    pub fetched_at: String,
    pub company_slug: String,
    pub categories: Vec<MenuCategory>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct UserConfig {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub phone: String,
    pub client_id: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_password: Option<String>,
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
