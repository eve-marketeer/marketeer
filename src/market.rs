//! EVE's market exports: where the game writes them, what is in one, and the price that
//! beats the best order in it.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// How many orders of each side the window lists.
const LISTED: usize = 10;

/// Which side of the market the price on the clipboard is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Mode {
    /// Undercut the cheapest sell order.
    #[default]
    Sell,
    /// Outbid the highest buy order.
    Buy,
}

impl Mode {
    pub(crate) fn other(self) -> Mode {
        match self {
            Mode::Sell => Mode::Buy,
            Mode::Buy => Mode::Sell,
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Mode::Sell => "Sell",
            Mode::Buy => "Buy",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Order {
    pub id: String,
    pub price: f64,
    pub volume: i64,
}

/// One export, read.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Export {
    pub type_id: Option<i64>,
    /// From the file name, until ESI's name is in.
    pub item: Option<String>,
    /// When the game wrote it, as the file name has it: `2025.11.29 125208`.
    pub exported: Option<String>,
    /// The cheapest sell orders, cheapest first.
    pub sells: Vec<Order>,
    /// The highest buy orders, highest first.
    pub buys: Vec<Order>,
    pub sell_count: usize,
    pub buy_count: usize,
}

impl Export {
    /// The best order on `mode`'s side.
    pub(crate) fn best(&self, mode: Mode) -> Option<f64> {
        match mode {
            Mode::Sell => self.sells.first().map(|o| o.price),
            Mode::Buy => self.buys.first().map(|o| o.price),
        }
    }

    /// The price that beats it by one tick.
    pub(crate) fn price(&self, mode: Mode) -> Option<f64> {
        self.best(mode).map(|price| beat(price, mode))
    }
}

/// Where the game keeps its market exports on this machine, the first that exists.
pub(crate) fn default_dir() -> Option<PathBuf> {
    candidates().into_iter().find(|p| p.is_dir())
}

fn candidates() -> Vec<PathBuf> {
    let tail = Path::new("EVE").join("logs").join("Marketlogs");
    let mut out = Vec::new();
    if let Some(docs) = dirs::document_dir() {
        out.push(docs.join(&tail));
    }
    if let Some(home) = dirs::home_dir() {
        // Windows with Documents in OneDrive.
        out.push(home.join("OneDrive").join("Documents").join(&tail));
        // macOS, the native client.
        out.push(
            home.join("Library/Application Support/EVE Online/p_drive/User/My Documents")
                .join(&tail),
        );
        // Linux, Steam's Proton prefix for EVE (app 8500).
        for steam in [
            ".steam/steam",
            ".local/share/Steam",
            ".var/app/com.valvesoftware.Steam/.local/share/Steam",
        ] {
            out.push(
                home.join(steam)
                    .join("steamapps/compatdata/8500/pfx/drive_c/users/steamuser/Documents")
                    .join(&tail),
            );
        }
    }
    out
}

/// The export written last.
pub(crate) fn latest(dir: &Path) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| is_export(path))
        .filter_map(|path| {
            let modified = path.metadata().ok()?.modified().ok()?;
            Some((modified, path))
        })
        .max_by(|a, b| a.0.cmp(&b.0))
        .map(|(_, path)| path)
}

pub(crate) fn is_export(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("txt"))
}

/// Read the newest export in `dir`, if there is one that can be read.
pub(crate) fn read_latest(dir: &Path) -> Option<Export> {
    let path = latest(dir)?;
    let text = std::fs::read_to_string(&path).ok()?;
    let name = path.file_stem()?.to_str()?;
    Some(parse(name, &text))
}

/// An export's contents. `name` is the file name without `.txt`.
///
/// The file is comma separated with a header row, nothing quoted, and a comma after the
/// last column: `price,volRemaining,typeID,range,orderID,...,bid,...,jumps,`.
pub(crate) fn parse(name: &str, text: &str) -> Export {
    let mut lines = text.lines();
    let header: Vec<&str> = lines
        .next()
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .collect();
    let column = |key: &str| header.iter().position(|h| *h == key);
    let (item, exported) = parse_name(name);
    let mut export = Export {
        item,
        exported,
        ..Export::default()
    };
    let (Some(price), Some(volume), Some(type_id), Some(order_id), Some(bid)) = (
        column("price"),
        column("volRemaining"),
        column("typeID"),
        column("orderID"),
        column("bid"),
    ) else {
        return export;
    };

    let mut sells = Vec::new();
    let mut buys = Vec::new();
    for line in lines {
        let cells: Vec<&str> = line.split(',').map(str::trim).collect();
        let cell = |index: usize| cells.get(index).copied();
        let Some(order_price) = cell(price).and_then(|s| s.parse::<f64>().ok()) else {
            continue;
        };
        // Written as a float: "3.0".
        let Some(order_volume) = cell(volume).and_then(|s| s.parse::<f64>().ok()) else {
            continue;
        };
        let (Some(id), Some(is_buy)) = (cell(order_id), cell(bid)) else {
            continue;
        };
        if export.type_id.is_none() {
            export.type_id = cell(type_id).and_then(|s| s.parse().ok());
        }
        #[expect(
            clippy::as_conversions,
            clippy::cast_possible_truncation,
            reason = "an order's volume is a whole number far below i64's range"
        )]
        let order = Order {
            id: id.to_string(),
            price: order_price,
            volume: order_volume as i64,
        };
        if is_buy.eq_ignore_ascii_case("true") {
            buys.push(order);
        } else {
            sells.push(order);
        }
    }
    sells.sort_by(|a, b| a.price.total_cmp(&b.price));
    buys.sort_by(|a, b| b.price.total_cmp(&a.price));
    export.sell_count = sells.len();
    export.buy_count = buys.len();
    sells.truncate(LISTED);
    buys.truncate(LISTED);
    export.sells = sells;
    export.buys = buys;
    export
}

/// The item and the time in `The Forge-Strong Mindflood Booster-2025.11.29 125208`. Item
/// names have dashes of their own, region names with a market do not.
fn parse_name(name: &str) -> (Option<String>, Option<String>) {
    let Some((rest, stamp)) = name.rsplit_once('-') else {
        return (Some(name.to_string()), None);
    };
    if !is_stamp(stamp) {
        return (Some(name.to_string()), None);
    }
    let item = rest.split_once('-').map_or(rest, |(_, item)| item);
    (Some(item.to_string()), Some(stamp.to_string()))
}

/// `2025.11.29 125208`.
fn is_stamp(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() == 17
        && bytes.iter().enumerate().all(|(i, b)| match i {
            4 | 7 => *b == b'.',
            10 => *b == b' ',
            _ => b.is_ascii_digit(),
        })
}

/// An export's time the way people write it: `Nov 29, 2025 at 12:52 PM`.
pub(crate) fn format_stamp(stamp: &str) -> String {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    if !is_stamp(stamp) {
        return stamp.to_string();
    }
    let number = |range: std::ops::Range<usize>| stamp[range].parse::<usize>().unwrap_or(0);
    let (year, month, day) = (number(0..4), number(5..7), number(8..10));
    let (hour, minute) = (number(11..13), number(13..15));
    let Some(month) = month.checked_sub(1).and_then(|m| MONTHS.get(m)) else {
        return stamp.to_string();
    };
    let half = if hour < 12 { "AM" } else { "PM" };
    let hour = match hour % 12 {
        0 => 12,
        h => h,
    };
    format!("{month} {day}, {year} at {hour}:{minute:02} {half}")
}

/// The smallest step the market takes at `price`: prices have four significant digits, so
/// 12,340 moves by 10 and 4.123 by 0.001, which ISK rounds up to a cent.
pub(crate) fn tick(price: f64) -> f64 {
    if price <= 0.0 {
        return 0.01;
    }
    #[expect(
        clippy::as_conversions,
        clippy::cast_possible_truncation,
        reason = "the exponent of an ISK price is a handful at most"
    )]
    let exponent = price.log10().floor() as i32;
    10_f64.powi(exponent - 3).max(0.01)
}

/// `price` beaten by one tick: lower to sell, higher to buy.
pub(crate) fn beat(price: f64, mode: Mode) -> f64 {
    if price <= 0.0 {
        return price;
    }
    let step = tick(price);
    let beaten = match mode {
        Mode::Sell => price - step,
        Mode::Buy => price + step,
    };
    // Whole cents: the subtraction leaves 12329.999999.
    ((beaten * 100.0).round() / 100.0).max(0.01)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXPORT: &str = "price,volRemaining,typeID,range,orderID,volEntered,minVolume,bid,issueDate,duration,stationID,regionID,solarSystemID,jumps,\n\
        12340.0,3.0,34,32767,1001,10,1,False,2025-11-29 10:00:00.000,90,60003760,10000002,30000142,0,\n\
        12350.0,5.0,34,32767,1002,10,1,False,2025-11-29 10:00:00.000,90,60003760,10000002,30000142,0,\n\
        11990.0,7.0,34,-1,1003,10,1,True,2025-11-29 10:00:00.000,90,60003760,10000002,30000142,0,\n\
        12010.0,2.0,34,-1,1004,10,1,True,2025-11-29 10:00:00.000,90,60003760,10000002,30000142,0,\n\
        0\n";

    #[test]
    fn reads_an_export() {
        let export = parse("The Forge-Tritanium-2025.11.29 125208", EXPORT);
        assert_eq!(export.type_id, Some(34));
        assert_eq!(export.item.as_deref(), Some("Tritanium"));
        assert_eq!(export.exported.as_deref(), Some("2025.11.29 125208"));
        assert_eq!((export.sell_count, export.buy_count), (2, 2));
        assert_eq!(export.sells[0].price, 12340.0);
        assert_eq!(export.sells[0].volume, 3);
        assert_eq!(export.buys[0].price, 12010.0);
        assert_eq!(export.price(Mode::Sell), Some(12330.0));
        assert_eq!(export.price(Mode::Buy), Some(12020.0));
    }

    #[test]
    fn an_empty_or_foreign_file_has_no_orders() {
        assert_eq!(parse("notes", "hello\nworld\n").sell_count, 0);
        assert_eq!(parse("notes", "").price(Mode::Sell), None);
    }

    #[test]
    fn item_names_keep_their_dashes() {
        let (item, stamp) =
            parse_name("The Forge-Zainou 'Gypsy' CPU Management EE-601-2025.11.29 125208");
        assert_eq!(
            item.as_deref(),
            Some("Zainou 'Gypsy' CPU Management EE-601")
        );
        assert_eq!(stamp.as_deref(), Some("2025.11.29 125208"));
        assert_eq!(parse_name("whatever"), (Some("whatever".into()), None));
    }

    #[test]
    fn ticks_follow_four_significant_digits() {
        assert_eq!(beat(12340.0, Mode::Sell), 12330.0);
        assert_eq!(beat(1_500_000_000.0, Mode::Sell), 1_499_000_000.0);
        assert_eq!(beat(998.9, Mode::Sell), 998.8);
        assert_eq!(beat(998.9, Mode::Buy), 999.0);
        assert_eq!(beat(5.12, Mode::Sell), 5.11);
        assert_eq!(beat(0.01, Mode::Sell), 0.01);
    }

    #[test]
    fn stamps_read_well() {
        assert_eq!(
            format_stamp("2025.11.29 125208"),
            "Nov 29, 2025 at 12:52 PM"
        );
        assert_eq!(format_stamp("2025.01.05 000900"), "Jan 5, 2025 at 12:09 AM");
        assert_eq!(format_stamp("garbage"), "garbage");
    }

    #[test]
    fn the_newest_export_wins() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a-Old-2025.11.29 125208.txt"), EXPORT).unwrap();
        std::fs::write(dir.path().join("ignored.csv"), "x").unwrap();
        let newer = dir.path().join("a-New-2025.11.29 125209.txt");
        std::fs::write(&newer, EXPORT).unwrap();
        let later = std::time::SystemTime::now() + std::time::Duration::from_secs(5);
        std::fs::File::options()
            .write(true)
            .open(&newer)
            .unwrap()
            .set_modified(later)
            .unwrap();
        assert_eq!(latest(dir.path()), Some(newer));
        assert_eq!(
            read_latest(dir.path()).unwrap().item.as_deref(),
            Some("New")
        );
    }
}
