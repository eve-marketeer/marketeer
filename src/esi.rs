//! What the app asks CCP for: an item's name, description and icon, its price history, and
//! for the bulk tool the ids behind names and Jita's sell orders. All of it is public, and
//! all of it blocks, so it runs off the UI thread.

use std::sync::LazyLock;
use std::time::Duration;

use serde::Deserialize;

const ESI: &str = "https://esi.evetech.net";
/// Static data: ESI itself no longer describes types.
const SDE: &str = "https://sde.evetech.net/tranquility/latest";
const IMAGES: &str = "https://images.evetech.net";
/// The API behavior this code was written against.
const COMPATIBILITY_DATE: &str = "2026-11-19";
const USER_AGENT: &str = concat!(
    "Marketeer/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/eve-marketeer/marketeer)"
);

/// The Forge, Jita's region.
const THE_FORGE: i64 = 10_000_002;
/// Jita IV - Moon 4 - Caldari Navy Assembly Plant.
const JITA_4_4: i64 = 60_003_760;
/// How many days of history the chart shows.
const HISTORY_DAYS: usize = 30;

static AGENT: LazyLock<ureq::Agent> = LazyLock::new(|| {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(15)))
        .user_agent(USER_AGENT)
        .build()
        .new_agent()
});

fn get(url: &str) -> Result<ureq::http::Response<ureq::Body>, String> {
    AGENT
        .get(url)
        .header("X-Compatibility-Date", COMPATIBILITY_DATE)
        .call()
        .map_err(|e| e.to_string())
}

/// An item as the game describes it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TypeInfo {
    pub name: String,
    pub description: String,
}

#[derive(Deserialize)]
struct SdeType {
    name: Localized,
    #[serde(default)]
    description: Localized,
}

#[derive(Deserialize, Default)]
struct Localized {
    #[serde(default)]
    en: String,
}

pub(crate) fn type_info(type_id: i64) -> Option<TypeInfo> {
    let found: SdeType = get(&format!("{SDE}/types/{type_id}.json"))
        .ok()?
        .body_mut()
        .read_json()
        .ok()?;
    Some(TypeInfo {
        name: found.name.en,
        description: plain_text(&found.description.en),
    })
}

/// The item's icon, as the image server sends it.
pub(crate) fn icon(type_id: i64) -> Option<Vec<u8>> {
    get(&format!("{IMAGES}/types/{type_id}/icon?size=64"))
        .ok()?
        .body_mut()
        .read_to_vec()
        .ok()
}

/// One day of an item's market in The Forge.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) struct Day {
    pub date: String,
    pub highest: f64,
    pub lowest: f64,
}

/// The last month of The Forge's history for an item, oldest first.
pub(crate) fn history(type_id: i64) -> Vec<Day> {
    let days: Vec<Day> = get(&format!(
        "{ESI}/markets/{THE_FORGE}/history?type_id={type_id}"
    ))
    .ok()
    .and_then(|mut response| response.body_mut().read_json().ok())
    .unwrap_or_default();
    let skip = days.len().saturating_sub(HISTORY_DAYS);
    days.into_iter().skip(skip).collect()
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct NamedType {
    pub id: i64,
    pub name: String,
}

#[derive(Deserialize)]
struct Ids {
    #[serde(default)]
    inventory_types: Vec<NamedType>,
}

/// The types with exactly these names. ESI leaves out the ones it does not know.
pub(crate) fn resolve(names: &[String]) -> Result<Vec<NamedType>, String> {
    let ids: Ids = AGENT
        .post(format!("{ESI}/universe/ids"))
        .header("X-Compatibility-Date", COMPATIBILITY_DATE)
        .send_json(names)
        .map_err(|e| format!("could not look the names up: {e}"))?
        .body_mut()
        .read_json()
        .map_err(|e| format!("could not read ESI's answer: {e}"))?;
    Ok(ids.inventory_types)
}

#[derive(Deserialize)]
struct RegionOrder {
    price: f64,
    location_id: i64,
}

/// The cheapest sell order for an item in Jita 4-4, or `None` when nobody sells it there.
pub(crate) fn lowest_jita_sell(type_id: i64) -> Result<Option<f64>, String> {
    let mut lowest: Option<f64> = None;
    let mut page = 1;
    loop {
        let mut response = get(&format!(
            "{ESI}/markets/{THE_FORGE}/orders?order_type=sell&type_id={type_id}&page={page}"
        ))?;
        let pages: u32 = response
            .headers()
            .get("x-pages")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);
        let orders: Vec<RegionOrder> =
            response.body_mut().read_json().map_err(|e| e.to_string())?;
        for order in orders {
            if order.location_id == JITA_4_4 && lowest.is_none_or(|l| order.price < l) {
                lowest = Some(order.price);
            }
        }
        if page >= pages {
            return Ok(lowest);
        }
        page += 1;
    }
}

/// A description without the game's markup: tags go, line breaks become spaces.
fn plain_text(description: &str) -> String {
    let mut out = String::with_capacity(description.len());
    let mut in_tag = false;
    let mut tag = String::new();
    for c in description.chars() {
        match c {
            '<' => {
                in_tag = true;
                tag.clear();
            }
            '>' if in_tag => {
                in_tag = false;
                if tag.to_ascii_lowercase().starts_with("br") {
                    out.push(' ');
                }
            }
            _ if in_tag => tag.push(c),
            '\r' | '\n' => out.push(' '),
            _ => out.push(c),
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptions_lose_their_markup() {
        assert_eq!(
            plain_text("Found in <a href=showinfo:1230>Veldspar</a>.<br>\r\nVery <b>hard</b>."),
            "Found in Veldspar. Very hard."
        );
        assert_eq!(plain_text("plain"), "plain");
    }
}
