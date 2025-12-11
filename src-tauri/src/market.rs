use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

pub static MARKET_STATE: Lazy<Mutex<MarketState>> = Lazy::new(|| {
    Mutex::new(MarketState {
        data: MarketData::empty(),
        mode: MarketMode::Sell,
    })
});

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketState {
    pub data: MarketData,
    pub mode: MarketMode,
}

impl MarketState {
    pub fn get_active_price(&self) -> Option<f64> {
        match self.mode {
            MarketMode::Sell => self.data.adjusted_sell,
            MarketMode::Buy => self.data.adjusted_buy,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MarketMode {
    Sell,
    Buy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketData {
    pub filename: Option<String>,
    pub type_id: Option<i32>,
    pub item_name: Option<String>,
    pub timestamp: Option<String>,
    pub cheapest_sell: Option<f64>,
    pub highest_buy: Option<f64>,
    pub adjusted_sell: Option<f64>,
    pub adjusted_buy: Option<f64>,
    pub sell_order_count: usize,
    pub buy_order_count: usize,
    pub sell_orders: Vec<MarketOrder>,
    pub buy_orders: Vec<MarketOrder>,
}

impl MarketData {
    pub fn empty() -> Self {
        Self {
            filename: None,
            type_id: None,
            item_name: None,
            timestamp: None,
            cheapest_sell: None,
            highest_buy: None,
            adjusted_sell: None,
            adjusted_buy: None,
            sell_order_count: 0,
            buy_order_count: 0,
            sell_orders: vec![],
            buy_orders: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketOrder {
    pub price: f64,
    pub vol_remaining: i64,
    pub type_id: i32,
    pub order_id: String,
    pub is_buy_order: bool,
    pub jumps: i32,
}

pub fn get_market_logs_path() -> Option<PathBuf> {
    dirs::document_dir().map(|d| d.join("EVE").join("logs").join("Marketlogs"))
}

pub fn find_latest_log_file() -> Option<PathBuf> {
    let logs_path = get_market_logs_path()?;
    
    if !logs_path.exists() {
        eprintln!("Market logs path does not exist: {:?}", logs_path);
        return None;
    }
    
    let result = fs::read_dir(&logs_path)
        .ok()?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.path().extension().map_or(false, |ext| ext == "txt")
        })
        .max_by_key(|entry| {
            entry.metadata().ok().and_then(|m| m.modified().ok())
        })
        .map(|entry| entry.path());
    
    if let Some(ref path) = result {
        eprintln!("Found latest log file: {:?}", path);
    } else {
        eprintln!("No log files found in {:?}", logs_path);
    }
    
    result
}

pub fn parse_latest_log() -> MarketData {
    let Some(file_path) = find_latest_log_file() else {
        return MarketData::empty();
    };
    
    parse_log_file(&file_path)
}

pub fn parse_log_file(file_path: &PathBuf) -> MarketData {
    let content = match fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to read file: {}", e);
            return MarketData::empty();
        }
    };
    
    // Use CSV reader for proper parsing
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .trim(csv::Trim::All)
        .from_reader(content.as_bytes());
    
    let headers: Vec<String> = match reader.headers() {
        Ok(h) => h.iter()
            .map(|s| s.trim_end_matches(',').trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        Err(e) => {
            eprintln!("Failed to read CSV headers: {}", e);
            return MarketData::empty();
        }
    };
    
    eprintln!("CSV Headers (filtered): {:?}", headers);
    
    let mut all_orders: Vec<MarketOrder> = vec![];
    
    for (i, result) in reader.records().enumerate() {
        let record = match result {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Failed to parse row {}: {}", i, e);
                continue;
            }
        };
        
        // Skip empty rows or rows with just "0"
        if record.len() == 0 || (record.len() == 1 && record.get(0) == Some("0")) {
            continue;
        }
        
        if let Some(order) = parse_order_from_record(&headers, &record, i) {
            all_orders.push(order);
        }
    }
    
    eprintln!("Parsed {} orders", all_orders.len());
    
    let mut sell_orders: Vec<MarketOrder> = all_orders
        .iter()
        .filter(|o| !o.is_buy_order)
        .cloned()
        .collect();
    sell_orders.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap());
    
    let mut buy_orders: Vec<MarketOrder> = all_orders
        .iter()
        .filter(|o| o.is_buy_order)
        .cloned()
        .collect();
    buy_orders.sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap());
    
    let cheapest_sell = sell_orders.first().map(|o| o.price);
    let highest_buy = buy_orders.first().map(|o| o.price);
    
    let adjusted_sell = cheapest_sell.map(|p| adjust_price(p, &MarketMode::Sell));
    let adjusted_buy = highest_buy.map(|p| adjust_price(p, &MarketMode::Buy));
    
    let type_id = all_orders.first().map(|o| o.type_id);
    let filename = file_path.file_name().and_then(|n| n.to_str()).map(String::from);
    let (item_name, timestamp) = parse_filename(filename.as_deref());
    
    let sell_order_count = sell_orders.len();
    let buy_order_count = buy_orders.len();
    
    eprintln!("Market data: typeId={:?}, sellOrders={}, buyOrders={}", type_id, sell_order_count, buy_order_count);
    
    MarketData {
        filename,
        type_id,
        item_name,
        timestamp,
        cheapest_sell,
        highest_buy,
        adjusted_sell,
        adjusted_buy,
        sell_order_count,
        buy_order_count,
        sell_orders: sell_orders.into_iter().take(10).collect(),
        buy_orders: buy_orders.into_iter().take(10).collect(),
    }
}

fn parse_order_from_record(headers: &[String], record: &csv::StringRecord, row_num: usize) -> Option<MarketOrder> {
    let get_value = |key: &str| -> Option<&str> {
        headers.iter().position(|h| h == key).and_then(|i| record.get(i))
    };
    
    let price_str = get_value("price");
    let vol_str = get_value("volRemaining");
    let type_id_str = get_value("typeID");
    let order_id_str = get_value("orderID");
    let bid_str = get_value("bid");
    let jumps_str = get_value("jumps");
    
    // Debug first few rows
    if row_num < 3 {
        eprintln!("Row {}: price={:?}, vol={:?}, typeID={:?}, bid={:?}", 
            row_num, price_str, vol_str, type_id_str, bid_str);
    }
    
    let price: f64 = price_str?.parse().ok()?;
    // volRemaining comes as float string like "3.0", parse as f64 then convert
    let vol_remaining: i64 = vol_str?.parse::<f64>().ok()? as i64;
    let type_id: i32 = type_id_str?.parse().ok()?;
    let order_id = order_id_str?.to_string();
    let is_buy_order = bid_str?.to_lowercase() == "true";
    let jumps = jumps_str.and_then(|s| s.parse().ok()).unwrap_or(0);
    
    Some(MarketOrder {
        price,
        vol_remaining,
        type_id,
        order_id,
        is_buy_order,
        jumps,
    })
}

fn parse_filename(filename: Option<&str>) -> (Option<String>, Option<String>) {
    let Some(filename) = filename else {
        return (None, None);
    };
    
    let name = filename.trim_end_matches(".txt");
    
    // Format: "Region-ItemName-Timestamp.txt"
    // e.g., "The Forge-Strong Mindflood Booster-2025.11.29 125208.txt"
    let Ok(re) = regex::Regex::new(r"^(.+)-(.+)-(\d{4}\.\d{2}\.\d{2}\s+\d+)$") else {
        return (Some(name.to_string()), None);
    };
    
    if let Some(caps) = re.captures(name) {
        let item_name = caps.get(2).map(|m| m.as_str().to_string());
        let timestamp = caps.get(3).map(|m| m.as_str().to_string());
        (item_name, timestamp)
    } else {
        (Some(name.to_string()), None)
    }
}

pub fn adjust_price(price: f64, mode: &MarketMode) -> f64 {
    if price <= 0.0 {
        return price;
    }
    
    let step = calculate_step(price);
    
    match mode {
        MarketMode::Sell => price - step,
        MarketMode::Buy => price + step,
    }
}

fn calculate_step(price: f64) -> f64 {
    if price <= 0.0 {
        return 1.0;
    }
    
    let num_digits = (price.floor() as i64).to_string().len();
    let exponent = (num_digits as i32) - 4;
    
    f64::max(1.0, 10_f64.powi(exponent))
}
