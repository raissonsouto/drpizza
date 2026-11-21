use chrono::{Datelike, NaiveDateTime, NaiveTime, Weekday};
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MenuCategory {
    pub id: u32,
    pub name: String,
    pub description: Option<String>,
    pub items: Vec<MenuItem>,
    #[serde(default, deserialize_with = "lenient_vec")]
    pub allowed_times: Vec<AllowedTime>,
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
    #[serde(default, deserialize_with = "lenient_vec")]
    pub allowed_times: Vec<AllowedTime>,
    #[serde(default, deserialize_with = "lenient_vec")]
    pub promotional_price_schedules: Vec<PromotionalPriceSchedule>,
    #[serde(default, deserialize_with = "lenient_vec")]
    pub combo_steps: Vec<ComboStep>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComboStep {
    #[serde(default)]
    pub price: Option<f64>,
}

/// Price as shown to the user, mirroring the cardapioweb catalog.
#[derive(Debug, Clone, PartialEq)]
pub struct DisplayPrice {
    /// Cheapest possible total (base + required add-on minimums).
    pub value: f64,
    /// True when choices can raise the price ("a partir de").
    pub starts_at: bool,
    /// Pre-promotion value, when a promotion is active.
    pub original: Option<f64>,
}

/// The API sends `""` or `null` instead of `[]` for some list fields.
fn lenient_vec<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum ListOrEmpty<T> {
        List(Vec<T>),
        Other(serde::de::IgnoredAny),
    }
    Ok(match Option::<ListOrEmpty<T>>::deserialize(deserializer)? {
        Some(ListOrEmpty::List(items)) => items,
        _ => Vec::new(),
    })
}

/// Availability window (e.g. Happy Hour). An empty list means always available.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AllowedTime {
    pub weekday: String,
    pub start_at: String,
    pub end_at: String,
}

/// Promotional price window. Without `start`/`end`, the whole `day` applies.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PromotionalPriceSchedule {
    pub day: String,
    #[serde(default)]
    pub start: Option<String>,
    #[serde(default)]
    pub end: Option<String>,
}

fn weekday_name(day: Weekday) -> &'static str {
    match day {
        Weekday::Mon => "monday",
        Weekday::Tue => "tuesday",
        Weekday::Wed => "wednesday",
        Weekday::Thu => "thursday",
        Weekday::Fri => "friday",
        Weekday::Sat => "saturday",
        Weekday::Sun => "sunday",
    }
}

fn parse_hhmm(value: &str) -> Option<NaiveTime> {
    NaiveTime::parse_from_str(value.trim(), "%H:%M").ok()
}

/// Matches `now` against a weekday + optional [start, end] window (end inclusive to the minute).
fn window_matches(day: &str, start: Option<&str>, end: Option<&str>, now: NaiveDateTime) -> bool {
    if !day.eq_ignore_ascii_case(weekday_name(now.weekday())) {
        return false;
    }
    let time = now.time();
    let after_start = start.and_then(parse_hhmm).map_or(true, |s| time >= s);
    let before_end = end
        .and_then(parse_hhmm)
        .map_or(true, |e| time < e + chrono::Duration::minutes(1));
    after_start && before_end
}

fn allowed_at(times: &[AllowedTime], now: NaiveDateTime) -> bool {
    times.is_empty()
        || times
            .iter()
            .any(|t| window_matches(&t.weekday, Some(&t.start_at), Some(&t.end_at), now))
}

impl MenuCategory {
    pub fn is_available_at(&self, now: NaiveDateTime) -> bool {
        allowed_at(&self.allowed_times, now)
    }
}

/// Keeps only categories/items orderable at `now`, dropping categories left empty.
pub fn available_menu_at(categories: &[MenuCategory], now: NaiveDateTime) -> Vec<MenuCategory> {
    categories
        .iter()
        .filter(|c| c.is_available_at(now))
        .filter_map(|c| {
            let mut category = c.clone();
            category.items.retain(|i| i.is_available_at(now));
            (!category.items.is_empty()).then_some(category)
        })
        .collect()
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AddOnGroup {
    pub id: u32,
    pub name: String,
    pub subitems: Vec<SubItem>,
    /// How selected subitem prices combine: "SUM" (default), "MEAN", "MIN" or "MAX".
    #[serde(default)]
    pub price_calculation_type: Option<String>,
    /// Cheapest contribution of this group given its required choices.
    #[serde(default)]
    pub minimum_price: Option<f64>,
}

impl AddOnGroup {
    pub fn is_flavor_group(&self) -> bool {
        self.name.to_lowercase().contains("sabor")
    }

    fn calculation_type(&self) -> String {
        self.price_calculation_type
            .as_deref()
            .unwrap_or("SUM")
            .to_ascii_uppercase()
    }

    /// Price contributed by `selected` subitems (repeat an entry for quantity > 1).
    /// Mirrors the cardapioweb client: SUM / MEAN (2 decimals) / MIN / MAX.
    pub fn combined_price(&self, selected: &[SubItem]) -> f64 {
        if selected.is_empty() {
            return 0.0;
        }
        let prices = selected.iter().map(|s| s.price);
        match self.calculation_type().as_str() {
            "MEAN" => round_to(prices.sum::<f64>() / selected.len() as f64, 2),
            "MIN" => prices.fold(f64::INFINITY, f64::min),
            "MAX" => prices.fold(f64::NEG_INFINITY, f64::max),
            _ => prices.sum(),
        }
    }

    /// Per-line `(price, total_price, add_on_selected_subitems_count)` for the
    /// order payload, given distinct `(subitem, quantity)` picks of this group.
    /// Non-SUM groups split the group price evenly across the distinct lines,
    /// exactly like the web client; the API rejects any other split.
    pub fn payload_lines(&self, picks: &[(&SubItem, u32)]) -> Vec<(f64, f64, Option<u32>)> {
        if self.calculation_type() == "SUM" {
            return picks
                .iter()
                .map(|(sub, qty)| (sub.price, round_to(sub.price * *qty as f64, 4), None))
                .collect();
        }
        let expanded: Vec<SubItem> = picks
            .iter()
            .flat_map(|(sub, qty)| std::iter::repeat((*sub).clone()).take(*qty as usize))
            .collect();
        let group_price = self.combined_price(&expanded);
        let selected_count = expanded.len() as u32;
        picks
            .iter()
            .map(|(_, qty)| {
                let total = round_to(group_price / picks.len() as f64, 4);
                let price = round_to(total / (*qty).max(1) as f64, 4);
                (price, total, Some(selected_count))
            })
            .collect()
    }
}

fn round_to(value: f64, decimals: i32) -> f64 {
    let factor = 10f64.powi(decimals);
    (value * factor).round() / factor
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
        self.price_at(chrono::Local::now().naive_local())
    }

    /// Promotional price only applies inside its schedule (if any); the API
    /// rejects orders priced with an expired promotion.
    pub fn price_at(&self, now: NaiveDateTime) -> f64 {
        match self.promotional_price {
            Some(promo) if self.promotional_price_active && self.promotion_applies_at(now) => promo,
            _ => self.price,
        }
    }

    fn promotion_applies_at(&self, now: NaiveDateTime) -> bool {
        self.promotional_price_schedules.is_empty()
            || self
                .promotional_price_schedules
                .iter()
                .any(|s| window_matches(&s.day, s.start.as_deref(), s.end.as_deref(), now))
    }

    pub fn is_available_at(&self, now: NaiveDateTime) -> bool {
        allowed_at(&self.allowed_times, now)
    }

    pub fn display_price(&self) -> DisplayPrice {
        self.display_price_at(chrono::Local::now().naive_local())
    }

    /// Same rule as the web catalog: `(promo or price) + Σ add_on.minimum_price`;
    /// combos show the sum of their steps.
    pub fn display_price_at(&self, now: NaiveDateTime) -> DisplayPrice {
        if self.kind == "combo" {
            let value = self.combo_steps.iter().filter_map(|s| s.price).sum();
            return DisplayPrice {
                value,
                starts_at: false,
                original: None,
            };
        }
        let add_on_minimum: f64 = self.add_ons.iter().filter_map(|a| a.minimum_price).sum();
        let base = self.price_at(now);
        let original = (base < self.price).then_some(self.price + add_on_minimum);
        DisplayPrice {
            value: base + add_on_minimum,
            starts_at: add_on_minimum > 0.0,
            original,
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
    /// Item's own price used in `price` (before flavors/crust/extras).
    pub base_price: f64,
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

#[cfg(test)]
mod tests {
    use super::MenuItem;

    #[test]
    fn menu_item_accepts_empty_string_or_null_for_list_fields() {
        let base = r#"{"id":1,"name":"X","custom_code":null,"description":null,"price":10.0,
            "promotional_price":null,"promotional_price_active":false,"kind":"regular_item",
            "print_area_id":null,"second_print_area_id":null,"add_ons":[]"#;
        for extra in [
            r#","promotional_price_schedules":"","allowed_times":null}"#,
            r#","promotional_price_schedules":[{"day":"friday"}],"allowed_times":[]}"#,
            "}",
        ] {
            let json = format!("{}{}", base, extra);
            let item: MenuItem = serde_json::from_str(&json).expect("item deveria parsear");
            assert!(item.allowed_times.is_empty());
        }
    }
}
