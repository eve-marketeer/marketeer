use clipboard_rs::{Clipboard, ClipboardContext};
use serde::{Deserialize, Serialize};
use std::time::Duration;

const BASE_URL: &str = "https://esi.evetech.net/latest";
const REGION_THE_FORGE: i32 = 10000002;
const JITA_4_4_STATION_ID: i64 = 60003760;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkResult {
    pub input_name: String,
    pub resolved_name: Option<String>,
    pub type_id: Option<i32>,
    pub qty: i32,
    pub lowest_sell: Option<f64>,
    pub undercut: Option<f64>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EsiIdsResponse {
    #[serde(default)]
    inventory_types: Vec<NamedId>,
}

#[derive(Debug, Deserialize, Clone)]
struct NamedId {
    id: i32,
    name: String,
}

#[derive(Debug, Deserialize)]
struct EsiOrder {
    price: f64,
    location_id: i64,
}

/// Parse a requirements list. Accepts lines like:
///   "Item Name\tQty"  or  "Item Name    Qty"  or  "Item Name"
/// Trailing numeric column = quantity (default 1).
pub fn parse_requirements(text: &str) -> Vec<(String, i32)> {
    let mut out = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        // Split on tabs first (EVE export format), then fallback to 2+ spaces
        let parts: Vec<&str> = if line.contains('\t') {
            line.split('\t')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect()
        } else {
            let mut v: Vec<&str> = Vec::new();
            let mut last_space = true;
            let mut start = 0;
            let bytes = line.as_bytes();
            let mut i = 0;
            while i < bytes.len() {
                if bytes[i] == b' ' && i + 1 < bytes.len() && bytes[i + 1] == b' ' {
                    if !last_space {
                        v.push(line[start..i].trim());
                    }
                    while i < bytes.len() && bytes[i] == b' ' {
                        i += 1;
                    }
                    start = i;
                    last_space = true;
                    continue;
                }
                last_space = false;
                i += 1;
            }
            if start < line.len() {
                v.push(line[start..].trim());
            }
            if v.is_empty() {
                vec![line]
            } else {
                v
            }
        };

        if parts.is_empty() {
            continue;
        }

        // If last token is a number, it's the qty
        let (name, qty) = if parts.len() >= 2 {
            if let Ok(n) = parts.last().unwrap().replace(',', "").parse::<i32>() {
                let name = parts[..parts.len() - 1].join(" ");
                (name, n)
            } else {
                (parts.join(" "), 1)
            }
        } else {
            (parts[0].to_string(), 1)
        };

        if !name.is_empty() {
            out.push((name, qty.max(1)));
        }
    }
    out
}

pub fn compute_undercut(price: f64) -> f64 {
    if price <= 0.0 {
        return price;
    }
    let exp = price.log10().floor() as i32;
    let step = 10f64.powi(exp - 3);
    price - step
}

async fn resolve_ids(client: &reqwest::Client, names: &[String]) -> Vec<NamedId> {
    // ESI /universe/ids/ accepts up to ~1000 names; we'll send all at once.
    let url = format!("{}/universe/ids/", BASE_URL);
    let resp = match client
        .post(&url)
        .json(names)
        .timeout(Duration::from_secs(15))
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    if !resp.status().is_success() {
        return Vec::new();
    }
    let parsed: EsiIdsResponse = match resp.json().await {
        Ok(p) => p,
        Err(_) => return Vec::new(),
    };
    parsed.inventory_types
}

async fn fetch_lowest_jita_sell(client: &reqwest::Client, type_id: i32) -> Option<f64> {
    let mut page = 1;
    let mut lowest: Option<f64> = None;
    loop {
        let url = format!(
            "{}/markets/{}/orders/?order_type=sell&type_id={}&page={}",
            BASE_URL, REGION_THE_FORGE, type_id, page
        );
        let resp = client
            .get(&url)
            .timeout(Duration::from_secs(15))
            .send()
            .await
            .ok()?;
        if !resp.status().is_success() {
            return lowest;
        }
        let pages: i32 = resp
            .headers()
            .get("x-pages")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);

        let orders: Vec<EsiOrder> = resp.json().await.ok()?;
        for o in orders {
            if o.location_id == JITA_4_4_STATION_ID {
                lowest = Some(match lowest {
                    Some(cur) if cur < o.price => cur,
                    _ => o.price,
                });
            }
        }
        if page >= pages {
            break;
        }
        page += 1;
    }
    lowest
}

pub async fn bulk_undercut(text: String) -> Vec<BulkResult> {
    let items = parse_requirements(&text);
    if items.is_empty() {
        return Vec::new();
    }

    let client = reqwest::Client::new();
    let names: Vec<String> = items.iter().map(|(n, _)| n.clone()).collect();
    let resolved = resolve_ids(&client, &names).await;

    // case-insensitive lookup
    let mut name_to_id: std::collections::HashMap<String, NamedId> =
        std::collections::HashMap::new();
    for r in resolved {
        name_to_id.insert(r.name.to_lowercase(), r);
    }

    // Fetch sell orders concurrently (bounded) per unique type_id
    let mut results: Vec<BulkResult> = Vec::with_capacity(items.len());
    let mut futs = Vec::new();
    for (name, qty) in &items {
        let lookup = name_to_id.get(&name.to_lowercase()).cloned();
        if let Some(info) = lookup {
            let client = client.clone();
            let tid = info.id;
            let resolved_name = info.name.clone();
            let input_name = name.clone();
            let qty = *qty;
            futs.push(tokio::spawn(async move {
                let lowest = fetch_lowest_jita_sell(&client, tid).await;
                let undercut = lowest.map(compute_undercut);
                BulkResult {
                    input_name,
                    resolved_name: Some(resolved_name),
                    type_id: Some(tid),
                    qty,
                    lowest_sell: lowest,
                    undercut,
                    error: if lowest.is_none() {
                        Some("No Jita 4-4 sell orders found".into())
                    } else {
                        None
                    },
                }
            }));
        } else {
            results.push(BulkResult {
                input_name: name.clone(),
                resolved_name: None,
                type_id: None,
                qty: *qty,
                lowest_sell: None,
                undercut: None,
                error: Some("Unknown item name".into()),
            });
        }
    }

    for f in futs {
        if let Ok(r) = f.await {
            results.push(r);
        }
    }

    // Preserve original input order
    let order: std::collections::HashMap<String, usize> = items
        .iter()
        .enumerate()
        .map(|(i, (n, _))| (n.to_lowercase(), i))
        .collect();
    results.sort_by_key(|r| {
        *order
            .get(&r.input_name.to_lowercase())
            .unwrap_or(&usize::MAX)
    });

    results
}

/// Format results as tab-separated sell-order lines:
/// `type_id\tname\tqty\tprice\tprice`
pub fn format_for_clipboard(results: &[BulkResult]) -> String {
    let mut out = String::new();
    for r in results {
        if let (Some(name), Some(price)) = (r.resolved_name.as_ref(), r.undercut) {
            out.push_str(&format!("{}\t{:.2}\n", name, price));
        }
    }
    out
}

pub fn copy_text_to_clipboard(text: String) -> bool {
    match ClipboardContext::new() {
        Ok(ctx) => ctx.set_text(text).is_ok(),
        Err(_) => false,
    }
}
