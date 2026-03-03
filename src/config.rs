use crate::api;
use crate::models::{LoyaltyReward, MenuCache, MenuCategory, UserConfig};
use crate::ui;
use chrono::Utc;
use std::fs;
use std::path::PathBuf;

// --- App Options ---

pub struct AppOptions {
    pub stateless: bool,
    pub no_cache: bool,
    pub unit_id: Option<usize>,
}

// --- File Paths ---

fn config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".drpizza")
}

fn menu_cache_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".drpizza_menu_cache.json")
}

// --- User Config ---

pub fn load_user_config(opts: &AppOptions) -> Option<UserConfig> {
    if opts.stateless {
        return None;
    }
    let path = config_path();
    let contents = fs::read_to_string(path).ok()?;
    serde_json::from_str(&contents).ok()
}

pub fn save_user_config(config: &UserConfig, opts: &AppOptions) {
    if opts.stateless {
        return;
    }
    let path = config_path();
    if let Ok(json) = serde_json::to_string_pretty(config) {
        if let Err(e) = fs::write(&path, json) {
            eprintln!("Erro ao salvar perfil: {}", e);
        }
    }
}

// --- Menu Cache ---

pub async fn get_menu_data(ctx: &api::ApiContext, opts: &AppOptions) -> Vec<MenuCategory> {
    let cache_file = menu_cache_path();

    // Try reading cache (skip if stateless or no-cache)
    if !opts.stateless && !opts.no_cache {
        if let Ok(contents) = fs::read_to_string(&cache_file) {
            if let Ok(cache) = serde_json::from_str::<MenuCache>(&contents) {
                if cache.company_slug == ctx.company_slug {
                    if let Ok(fetched) = chrono::DateTime::parse_from_rfc3339(&cache.fetched_at) {
                        let age = Utc::now().signed_duration_since(fetched);
                        if age.num_minutes() < 30 {
                            return cache.categories;
                        }
                    }
                }
            }
        }
    }

    // Fetch from API
    let sp = ui::Spinner::new("Carregando cardápio...");
    match api::fetch_menu(ctx).await {
        Ok(categories) => {
            sp.stop();
            // Write cache (skip if stateless)
            if !opts.stateless {
                let cache = MenuCache {
                    fetched_at: Utc::now().to_rfc3339(),
                    company_slug: ctx.company_slug.clone(),
                    categories: categories.clone(),
                };
                if let Ok(json) = serde_json::to_string_pretty(&cache) {
                    let _ = fs::write(&cache_file, json);
                }
            }
            categories
        }
        Err(e) => {
            drop(sp);
            eprintln!("Erro ao buscar cardápio da API: {}", e);
            // Fall back to stale cache (skip if stateless)
            if !opts.stateless {
                if let Ok(contents) = fs::read_to_string(&cache_file) {
                    if let Ok(cache) = serde_json::from_str::<MenuCache>(&contents) {
                        return cache.categories;
                    }
                }
            }
            Vec::new()
        }
    }
}

// --- Loyalty ---

pub fn get_loyalty_rewards() -> Vec<LoyaltyReward> {
    let data = r#"
    [{"id":96459,"name":"Guaraná 1L","kind":"item","points_per_currency_unit":65,"active":true},{"id":70316,"name":"Desconto R$ 3,00","kind":"discount","points_quantity_required":300,"active":true}]
    "#;
    serde_json::from_str(data).expect("Erro ao processar recompensas")
}
