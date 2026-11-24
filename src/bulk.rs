//! The bulk tool: a pasted list of items and quantities, each priced one tick under Jita
//! 4-4's cheapest sell order.

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::esi;
use crate::market::{self, Mode};

/// How many items' orders are fetched at once.
const WORKERS: usize = 6;

/// One line of the list, priced or not.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Row {
    /// The name as ESI spells it, or as it was pasted when ESI does not know it.
    pub name: String,
    pub quantity: i64,
    pub lowest_sell: Option<f64>,
    pub undercut: Option<f64>,
    pub problem: Option<String>,
}

/// What the rows add up to.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct Summary {
    pub items: usize,
    pub units: i64,
    pub total: f64,
    pub missing: usize,
}

pub(crate) fn summary(rows: &[Row]) -> Summary {
    let mut out = Summary::default();
    for row in rows {
        match row.undercut {
            Some(price) => {
                out.items += 1;
                out.units += row.quantity;
                #[expect(
                    clippy::as_conversions,
                    clippy::cast_precision_loss,
                    reason = "a quantity is far below the 2^53 a f64 holds exactly"
                )]
                let quantity = row.quantity as f64;
                out.total += price * quantity;
            }
            None => out.missing += 1,
        }
    }
    out
}

/// The items and quantities in a pasted list. A line is a name, then optionally a
/// quantity after a tab, spaces or an `x`: the game's inventory and multibuy copies, and
/// lists typed by hand. Without one the quantity is 1.
pub(crate) fn parse(text: &str) -> Vec<(String, i64)> {
    text.lines().filter_map(parse_line).collect()
}

fn parse_line(line: &str) -> Option<(String, i64)> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    // The game's copies are tab separated, with more columns after the quantity.
    if let Some((name, rest)) = line.split_once('\t') {
        let quantity = rest.split('\t').find_map(quantity).unwrap_or(1);
        let name = name.trim();
        return (!name.is_empty()).then(|| (name.to_string(), quantity));
    }
    if let Some((name, last)) = line.rsplit_once(' ')
        && let Some(quantity) = quantity(last.strip_prefix('x').unwrap_or(last))
        && !name.trim().is_empty()
    {
        return Some((name.trim().to_string(), quantity));
    }
    Some((line.to_string(), 1))
}

/// `7`, `1,000` or `1.000` as a quantity.
fn quantity(text: &str) -> Option<i64> {
    let text = text.trim();
    if text.is_empty()
        || !text
            .chars()
            .all(|c| c.is_ascii_digit() || c == ',' || c == '.')
    {
        return None;
    }
    let digits: String = text.chars().filter(char::is_ascii_digit).collect();
    digits.parse::<i64>().ok().map(|n| n.max(1))
}

/// Price every line of `text`. Blocking: one request for the names, then one or more per
/// distinct item.
pub(crate) fn run(text: &str) -> Result<Vec<Row>, String> {
    let lines = parse(text);
    if lines.is_empty() {
        return Ok(Vec::new());
    }
    let mut names: Vec<String> = lines.iter().map(|(name, _)| name.clone()).collect();
    names.sort_unstable();
    names.dedup();
    let known: HashMap<String, esi::NamedType> = esi::resolve(&names)?
        .into_iter()
        .map(|t| (t.name.to_lowercase(), t))
        .collect();

    let mut ids: Vec<i64> = known.values().map(|t| t.id).collect();
    ids.sort_unstable();
    ids.dedup();
    let prices = lowest_sells(&ids);

    Ok(lines
        .into_iter()
        .map(|(name, quantity)| {
            let Some(found) = known.get(&name.to_lowercase()) else {
                return Row {
                    name,
                    quantity,
                    lowest_sell: None,
                    undercut: None,
                    problem: Some("Unknown item name".into()),
                };
            };
            let (lowest_sell, problem) = match prices.get(&found.id) {
                Some(Ok(Some(price))) => (Some(*price), None),
                Some(Ok(None)) => (None, Some("No sell orders in Jita 4-4".to_string())),
                Some(Err(err)) => (None, Some(format!("Could not get the orders: {err}"))),
                None => (None, Some("Could not get the orders".to_string())),
            };
            Row {
                name: found.name.clone(),
                quantity,
                lowest_sell,
                undercut: lowest_sell.map(|price| market::beat(price, Mode::Sell)),
                problem,
            }
        })
        .collect())
}

/// Jita's cheapest sell order per type, a few types at a time.
fn lowest_sells(ids: &[i64]) -> HashMap<i64, Result<Option<f64>, String>> {
    let next = AtomicUsize::new(0);
    let found = Mutex::new(HashMap::with_capacity(ids.len()));
    std::thread::scope(|scope| {
        for _ in 0..WORKERS.min(ids.len()) {
            scope.spawn(|| {
                while let Some(&id) = ids.get(next.fetch_add(1, Ordering::Relaxed)) {
                    let price = esi::lowest_jita_sell(id);
                    if let Ok(mut found) = found.lock() {
                        found.insert(id, price);
                    }
                }
            });
        }
    });
    found.into_inner().unwrap_or_default()
}

/// The priced rows as `name<tab>price` lines.
pub(crate) fn for_clipboard(rows: &[Row]) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    for row in rows {
        if let Some(price) = row.undercut {
            let _ = writeln!(out, "{}\t{price:.2}", row.name);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_ways_a_list_is_written() {
        let text = "Multispectrum Shield Hardener II\t7\n\
                    1600mm Steel Plates II    2\n\
                    Tritanium 1,000\n\
                    Nanite Repair Paste x50\n\
                    \n\
                    Damage Control II\n\
                    Hammerhead II\t5\tCombat Drone\tMedium\n";
        assert_eq!(
            parse(text),
            vec![
                ("Multispectrum Shield Hardener II".to_string(), 7),
                ("1600mm Steel Plates II".to_string(), 2),
                ("Tritanium".to_string(), 1000),
                ("Nanite Repair Paste".to_string(), 50),
                ("Damage Control II".to_string(), 1),
                ("Hammerhead II".to_string(), 5),
            ]
        );
    }

    fn row(name: &str, quantity: i64, undercut: Option<f64>) -> Row {
        Row {
            name: name.into(),
            quantity,
            lowest_sell: undercut,
            undercut,
            problem: None,
        }
    }

    #[test]
    fn sums_and_copies_only_what_has_a_price() {
        let rows = [
            row("A", 2, Some(10.0)),
            row("B", 5, None),
            row("C", 1, Some(0.5)),
        ];
        let sum = summary(&rows);
        assert_eq!((sum.items, sum.units, sum.missing), (2, 3, 1));
        assert!((sum.total - 20.5).abs() < 1e-9);
        assert_eq!(for_clipboard(&rows), "A\t10.00\nC\t0.50\n");
    }
}

#[cfg(test)]
mod live {
    /// Talks to ESI. Run with `cargo test -- --ignored`.
    #[test]
    #[ignore = "needs the network"]
    fn prices_a_list_against_jita() {
        let rows = super::run("Tritanium\t1,000\nDamage Control II 2\nNot An Item x3\n").unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].name, "Tritanium");
        assert!(rows[0].undercut.is_some_and(|p| p > 0.0), "{rows:?}");
        assert!(rows[1].undercut < rows[1].lowest_sell, "{rows:?}");
        assert_eq!(rows[2].problem.as_deref(), Some("Unknown item name"));
    }
}
