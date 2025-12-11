use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

const BASE_URL: &str = "https://esi.evetech.net/latest";
const CACHE_TTL: Duration = Duration::from_secs(3600); // 1 hour
const DEFAULT_REGION_ID: i32 = 10000002; // The Forge (Jita's region)

static TYPE_CACHE: Lazy<Mutex<HashMap<i32, CacheEntry<TypeInfo>>>> = 
    Lazy::new(|| Mutex::new(HashMap::new()));

static HISTORY_CACHE: Lazy<Mutex<HashMap<i32, CacheEntry<Vec<MarketHistory>>>>> = 
    Lazy::new(|| Mutex::new(HashMap::new()));

struct CacheEntry<T> {
    data: T,
    expires_at: Instant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeInfo {
    pub type_id: i32,
    pub name: String,
    pub description: String,
    pub icon_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EsiTypeResponse {
    pub type_id: i32,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketHistory {
    pub date: String,
    pub average: f64,
    pub highest: f64,
    pub lowest: f64,
    pub order_count: i64,
    pub volume: i64,
}

#[derive(Debug, Deserialize)]
struct EsiHistoryResponse {
    date: String,
    average: f64,
    highest: f64,
    lowest: f64,
    order_count: i64,
    volume: i64,
}

pub async fn get_type_info(type_id: i32) -> Option<TypeInfo> {
    // Check cache first
    {
        let cache = TYPE_CACHE.lock();
        if let Some(entry) = cache.get(&type_id) {
            if entry.expires_at > Instant::now() {
                return Some(entry.data.clone());
            }
        }
    }
    
    // Fetch from API
    let url = format!("{}/universe/types/{}/", BASE_URL, type_id);
    
    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .ok()?;
    
    if !response.status().is_success() {
        return None;
    }
    
    let esi_type: EsiTypeResponse = response.json().await.ok()?;
    
    let type_info = TypeInfo {
        type_id: esi_type.type_id,
        name: esi_type.name,
        description: clean_description(&esi_type.description),
        icon_url: format!("https://images.evetech.net/types/{}/icon?size=64", type_id),
    };
    
    // Cache the result
    {
        let mut cache = TYPE_CACHE.lock();
        cache.insert(type_id, CacheEntry {
            data: type_info.clone(),
            expires_at: Instant::now() + CACHE_TTL,
        });
    }
    
    Some(type_info)
}

pub async fn get_market_history(type_id: i32) -> Option<Vec<MarketHistory>> {
    // Check cache first
    {
        let cache = HISTORY_CACHE.lock();
        if let Some(entry) = cache.get(&type_id) {
            if entry.expires_at > Instant::now() {
                return Some(entry.data.clone());
            }
        }
    }
    
    let url = format!(
        "{}/markets/{}/history/?type_id={}",
        BASE_URL, DEFAULT_REGION_ID, type_id
    );
    
    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .ok()?;
    
    if !response.status().is_success() {
        return None;
    }
    
    let esi_history: Vec<EsiHistoryResponse> = response.json().await.ok()?;
    
    let history: Vec<MarketHistory> = esi_history
        .into_iter()
        .map(|h| MarketHistory {
            date: h.date,
            average: h.average,
            highest: h.highest,
            lowest: h.lowest,
            order_count: h.order_count,
            volume: h.volume,
        })
        .collect();
    
    // Get last 30 days
    let history: Vec<MarketHistory> = history.into_iter().rev().take(30).rev().collect();
    
    // Cache the result
    {
        let mut cache = HISTORY_CACHE.lock();
        cache.insert(type_id, CacheEntry {
            data: history.clone(),
            expires_at: Instant::now() + CACHE_TTL,
        });
    }
    
    Some(history)
}

fn clean_description(description: &str) -> String {
    // Remove EVE's HTML-like tags
    let re = regex::Regex::new(r"<[^>]+>").unwrap();
    re.replace_all(description, "").to_string()
}

