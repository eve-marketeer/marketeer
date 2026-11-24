//! Pieces the pages share: how numbers read, panels, key caps, and the price chart.

use std::cell::Cell;
use std::rc::Rc;

use gpui_kit::component::Theme;
use gpui_kit::{
    Bounds, Div, Hsla, IntoElement, ParentElement, PathBuilder, Pixels, SharedString, Styled,
    canvas, div, fill, point, px, size, svg,
};

use crate::esi::Day;
use crate::theme;

/// `1234567.5` as `1,234,567.50`.
pub(crate) fn isk(value: f64) -> String {
    let text = format!("{:.2}", value.abs());
    let (whole, cents) = text.split_once('.').unwrap_or((&text, "00"));
    let sign = if value < 0.0 { "-" } else { "" };
    format!("{sign}{}.{cents}", grouped(whole))
}

/// `1234567` as `1,234,567`.
pub(crate) fn count(value: i64) -> String {
    let sign = if value < 0 { "-" } else { "" };
    format!("{sign}{}", grouped(&value.unsigned_abs().to_string()))
}

fn grouped(digits: &str) -> String {
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// `1234567` as `1.2M`.
pub(crate) fn compact(value: f64) -> String {
    let magnitude = value.abs();
    let (scaled, unit) = if magnitude >= 1e12 {
        (value / 1e12, "T")
    } else if magnitude >= 1e9 {
        (value / 1e9, "B")
    } else if magnitude >= 1e6 {
        (value / 1e6, "M")
    } else if magnitude >= 1e3 {
        (value / 1e3, "K")
    } else {
        (value, "")
    };
    let text = format!("{scaled:.1}");
    let text = text.strip_suffix(".0").unwrap_or(&text);
    format!("{text}{unit}")
}

/// `2026-11-12` as `Nov 12`.
pub(crate) fn short_date(date: &str) -> String {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let mut parts = date.split('-');
    let (_, Some(month), Some(day)) = (parts.next(), parts.next(), parts.next()) else {
        return date.to_string();
    };
    let (Some(month), Ok(day)) = (
        month
            .parse::<usize>()
            .ok()
            .and_then(|m| m.checked_sub(1))
            .and_then(|m| MONTHS.get(m)),
        day.parse::<u8>(),
    ) else {
        return date.to_string();
    };
    format!("{month} {day}")
}

/// Small muted text: a label over something, or a note under it.
pub(crate) fn caption(text: impl Into<SharedString>, theme: &Theme) -> Div {
    div()
        .text_xs()
        .text_color(theme.muted_foreground)
        .child(text.into())
}

/// An icon from the kit's set, `size` pixels square.
pub(crate) fn icon(path: &'static str, size: f32, color: Hsla) -> impl IntoElement {
    svg()
        .flex_none()
        .path(path)
        .size(px(size))
        .text_color(color)
}

/// A shortcut as key caps.
pub(crate) fn key_caps(keys: &[String], theme: &Theme) -> Div {
    div()
        .flex()
        .items_center()
        .gap_1()
        .children(keys.iter().map(|key| {
            div()
                .px_2()
                .py(px(1.0))
                .rounded(px(4.0))
                .border_1()
                .border_color(theme.border)
                .bg(theme.muted)
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(key.clone())
        }))
}

/// The price a line of the chart draws for a day.
type Series = fn(&Day) -> f64;

/// Where the chart was drawn last, so the mouse can be matched to a day.
pub(crate) type ChartBounds = Rc<Cell<Option<Bounds<Pixels>>>>;

/// The day under `x`, given where the chart is.
pub(crate) fn day_at(bounds: Bounds<Pixels>, x: Pixels, days: usize) -> Option<usize> {
    if days < 2 {
        return None;
    }
    let width = f32::from(bounds.size.width);
    if width <= 0.0 {
        return None;
    }
    let along = (f32::from(x - bounds.origin.x) / width).clamp(0.0, 1.0);
    #[expect(
        clippy::as_conversions,
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a month of days, and `along` is between 0 and 1"
    )]
    let index = (along * (days - 1) as f32).round() as usize;
    Some(index.min(days - 1))
}

/// The lowest and highest price in `days`, with a little room above and below.
pub(crate) fn price_range(days: &[Day]) -> (f64, f64) {
    let low = days.iter().map(|d| d.lowest).fold(f64::INFINITY, f64::min);
    let high = days
        .iter()
        .map(|d| d.highest)
        .fold(f64::NEG_INFINITY, f64::max);
    if !low.is_finite() || !high.is_finite() {
        return (0.0, 1.0);
    }
    let room = ((high - low) * 0.08).max(high.abs() * 0.001).max(0.01);
    ((low - room).max(0.0), high + room)
}

/// Each day's highest and lowest price as two lines over three grid lines, with a marker
/// on the hovered day.
pub(crate) fn price_chart(
    days: Rc<[Day]>,
    hovered: Option<usize>,
    slot: ChartBounds,
    theme: &Theme,
) -> impl IntoElement {
    let grid = theme::hairline();
    let marker = theme.muted_foreground.opacity(0.5);
    let background = theme.background;
    canvas(
        move |bounds, _, _| slot.set(Some(bounds)),
        move |bounds, (), window, _| {
            let (low, high) = price_range(&days);
            let (left, top) = (bounds.origin.x, bounds.origin.y);
            let (width, height) = (bounds.size.width, bounds.size.height);
            #[expect(
                clippy::as_conversions,
                clippy::cast_precision_loss,
                reason = "a month of days"
            )]
            let x = |index: usize| left + width * (index as f32 / (days.len().max(2) - 1) as f32);
            #[expect(
                clippy::as_conversions,
                clippy::cast_possible_truncation,
                reason = "a share of the chart's height, between 0 and 1"
            )]
            let y = |price: f64| top + height * (1.0 - ((price - low) / (high - low)) as f32);

            for line in 0..3_u8 {
                let at = top + (height - px(1.0)) * (f32::from(line) / 2.0);
                window.paint_quad(fill(
                    Bounds::new(point(left, at), size(width, px(1.0))),
                    grid,
                ));
            }
            if let Some(index) = hovered.filter(|i| *i < days.len()) {
                window.paint_quad(fill(
                    Bounds::new(point(x(index), top), size(px(1.0), height)),
                    marker,
                ));
            }
            // A soft wash under the highs, so the chart reads as a shape and not two wires.
            let mut area = PathBuilder::fill();
            area.move_to(point(left, top + height));
            for (index, day) in days.iter().enumerate() {
                area.line_to(point(x(index), y(day.highest)));
            }
            area.line_to(point(left + width, top + height));
            area.close();
            if let Ok(area) = area.build() {
                window.paint_path(area, theme::sell().opacity(0.06));
            }
            let series: [(Series, Hsla); 2] = [
                (|day| day.highest, theme::sell()),
                (|day| day.lowest, theme::buy()),
            ];
            for (value, color) in series {
                let mut path = PathBuilder::stroke(px(2.0));
                for (index, day) in days.iter().enumerate() {
                    let to = point(x(index), y(value(day)));
                    if index == 0 {
                        path.move_to(to);
                    } else {
                        path.line_to(to);
                    }
                }
                if let Ok(path) = path.build() {
                    window.paint_path(path, color);
                }
                if let Some(day) = hovered.and_then(|i| days.get(i).map(|d| (i, d))) {
                    let center = point(x(day.0), y(value(day.1)));
                    let dot = |radius: f32, color: Hsla| {
                        fill(
                            Bounds::new(
                                point(center.x - px(radius), center.y - px(radius)),
                                size(px(radius * 2.0), px(radius * 2.0)),
                            ),
                            color,
                        )
                        .corner_radii(px(radius))
                    };
                    window.paint_quad(dot(5.0, background));
                    window.paint_quad(dot(3.5, color));
                }
            }
        },
    )
    .size_full()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_read_well() {
        assert_eq!(isk(1_234_567.5), "1,234,567.50");
        assert_eq!(isk(999.999), "1,000.00");
        assert_eq!(isk(0.01), "0.01");
        assert_eq!(count(1_234_567), "1,234,567");
        assert_eq!(count(12), "12");
        assert_eq!(compact(1_234_567.0), "1.2M");
        assert_eq!(compact(2_000_000_000.0), "2B");
        assert_eq!(compact(999.0), "999");
        assert_eq!(compact(12_340.0), "12.3K");
        assert_eq!(short_date("2026-11-02"), "Nov 2");
        assert_eq!(short_date("soon"), "soon");
    }

    #[test]
    fn the_price_range_leaves_room() {
        let day = |lowest, highest| Day {
            date: String::new(),
            highest,
            lowest,
        };
        let (low, high) = price_range(&[day(100.0, 110.0), day(90.0, 120.0)]);
        assert!(low < 90.0 && high > 120.0);
        let (low, high) = price_range(&[day(5.0, 5.0)]);
        assert!(low < 5.0 && high > 5.0);
        assert_eq!(price_range(&[]), (0.0, 1.0));
    }
}
