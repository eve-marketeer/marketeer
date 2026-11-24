//! What the window draws. Nothing is boxed: the price to paste is the biggest thing on the
//! page, the side it is for tints the top of the window, and the rest is set apart by
//! space and hairlines.

use std::rc::Rc;

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, Textarea};
use gpui_kit::component::{ActiveTheme as _, Disableable as _, Icon, Sizable as _, Theme};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, Context, Div, FontWeight, Hsla, InteractiveElement as _, IntoElement, KeyDownEvent,
    MouseMoveEvent, ParentElement as _, Render, SharedString, StatefulInteractiveElement as _,
    Styled as _, Window, div, img, linear_color_stop, linear_gradient, px, relative,
};

use super::{COPIED_FOR, Marketeer, Page, Updater};
use crate::bulk::{self, Row};
use crate::hotkey;
use crate::market::{self, Export, Mode, Order};
use crate::theme;
use crate::ui::{self, caption, icon, key_caps};
use crate::update;

/// How wide the page's column gets in a wide window.
const COLUMN: f32 = 860.0;

fn button_icon(path: &'static str) -> Icon {
    Icon::empty().path(path)
}

impl Render for Marketeer {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let tint = theme::side(self.config.mode);
        let page = match self.page {
            Page::Market => self.market_page(&theme, cx),
            Page::Bulk => self.bulk_page(&theme, cx),
            Page::Settings => self.settings_page(&theme, cx),
        };
        div()
            .id("root")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                if this.capturing {
                    this.captured(event, cx);
                } else {
                    this.page_key(event, cx);
                }
            }))
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .bg(theme.background)
            .text_color(theme.foreground)
            // The side the clipboard price is for, as a glow from the top of the window.
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .right_0()
                    .h(px(300.0))
                    .bg(linear_gradient(
                        180.0,
                        linear_color_stop(tint.opacity(0.11), 0.0),
                        linear_color_stop(tint.opacity(0.0), 1.0),
                    )),
            )
            .child(self.header(&theme, cx))
            .child(
                div()
                    .id("page")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(
                        div().flex().justify_center().px_6().pb_8().child(
                            div()
                                .w_full()
                                .max_w(px(COLUMN))
                                .flex()
                                .flex_col()
                                .gap_3()
                                .children(self.update_bar(&theme, cx))
                                .children(self.problem_bars(&theme, cx))
                                .child(page),
                        ),
                    ),
            )
    }
}

impl Marketeer {
    /// The mark, the pages, and the switch between selling and buying.
    fn header(&self, theme: &Theme, cx: &mut Context<'_, Self>) -> Div {
        let mode = self.config.mode;
        let tabs = Page::ALL.into_iter().map(|page| {
            let selected = self.page == page;
            div()
                .id(page.label())
                .relative()
                .flex()
                .items_center()
                .h_full()
                .px_3()
                .text_sm()
                .cursor_pointer()
                .map(|d| {
                    if selected {
                        d.font_weight(FontWeight::MEDIUM)
                            .text_color(theme.foreground)
                            .child(
                                div()
                                    .absolute()
                                    .left(px(12.0))
                                    .right(px(12.0))
                                    .bottom(px(10.0))
                                    .h(px(2.0))
                                    .rounded_full()
                                    .bg(theme::side(mode)),
                            )
                    } else {
                        d.text_color(theme.muted_foreground)
                            .hover(|s| s.text_color(theme.foreground))
                    }
                })
                .child(page.label())
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.page = page;
                    this.capturing = false;
                    cx.notify();
                }))
        });
        let side = |side: Mode| {
            let active = mode == side;
            let color = theme::side(side);
            div()
                .id(side.label())
                .flex()
                .items_center()
                .gap_1()
                .h(px(24.0))
                .px_3()
                .rounded_full()
                .text_xs()
                .font_weight(FontWeight::MEDIUM)
                .cursor_pointer()
                .map(|d| {
                    if active {
                        d.bg(color.opacity(0.2)).text_color(color)
                    } else {
                        d.text_color(theme.muted_foreground)
                            .hover(|s| s.text_color(theme.foreground))
                    }
                })
                .child(icon(
                    mode_icon(side),
                    12.0,
                    if active {
                        color
                    } else {
                        theme.muted_foreground
                    },
                ))
                .child(side.label())
                .on_click(cx.listener(move |this, _, _, cx| this.set_mode(side, cx)))
        };
        div()
            .flex_none()
            .flex()
            .items_center()
            .h(px(52.0))
            .px_6()
            .child(icon("logo.svg", 17.0, theme::side(mode)))
            .child(
                div()
                    .ml_2()
                    .mr_4()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Marketeer"),
            )
            .children(tabs)
            .child(div().flex_1())
            .child(
                div()
                    .flex()
                    .p(px(2.0))
                    .rounded_full()
                    .bg(gpui_kit::white().opacity(0.06))
                    .child(side(Mode::Sell))
                    .child(side(Mode::Buy)),
            )
    }

    /// A slim tinted bar across the page: an icon, what it says, and what to do about it.
    fn bar(path: &'static str, color: Hsla) -> Div {
        div()
            .flex_none()
            .flex()
            .items_center()
            .gap_3()
            .px_4()
            .py_2()
            .rounded(px(10.0))
            .bg(color.opacity(0.1))
            .text_sm()
            .child(icon(path, 15.0, color))
    }

    /// Says a new version is out.
    fn update_bar(&self, theme: &Theme, cx: &mut Context<'_, Self>) -> Option<Div> {
        let available = self.update.as_ref()?;
        let installing = self.updater == Updater::Installing;
        let (label, text) = if installing {
            ("Updating…", "Downloading, then Marketeer restarts.")
        } else if available.installer.is_some() {
            ("Update and restart", "Installs in place.")
        } else {
            ("Download", "Get it from the release page.")
        };
        let page = available.page.clone();
        Some(
            Self::bar("icons/download.svg", theme.primary)
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .items_baseline()
                        .gap_2()
                        .child(
                            div()
                                .font_weight(FontWeight::MEDIUM)
                                .child(format!("Marketeer {} is out", available.version)),
                        )
                        .child(caption(text, theme)),
                )
                .when(available.installer.is_some() && !installing, |d| {
                    d.child(
                        Button::new("update-notes")
                            .label("What's new")
                            .small()
                            .ghost()
                            .on_click(move |_, _, cx| cx.open_url(&page)),
                    )
                })
                .child(
                    Button::new("update-install")
                        .label(label)
                        .small()
                        .primary()
                        .loading(installing)
                        .disabled(installing)
                        .on_click(cx.listener(|this, _, _, cx| this.install_update(cx))),
                ),
        )
    }

    /// What went wrong, each until dismissed.
    fn problem_bars(&self, theme: &Theme, cx: &mut Context<'_, Self>) -> Vec<Div> {
        self.problems
            .iter()
            .enumerate()
            .map(|(index, problem)| {
                Self::bar("icons/circle-alert.svg", theme.danger)
                    .child(div().flex_1().min_w_0().child(problem.clone()))
                    .child(
                        Button::new(("dismiss", index))
                            .icon(button_icon("icons/x.svg"))
                            .small()
                            .ghost()
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if index < this.problems.len() {
                                    this.problems.remove(index);
                                }
                                cx.notify();
                            })),
                    )
            })
            .collect()
    }

    // Market.

    fn market_page(&self, theme: &Theme, cx: &mut Context<'_, Self>) -> AnyElement {
        let Some(export) = self.export.as_ref().filter(|e| e.type_id.is_some()) else {
            return self.empty_state(theme, cx).into_any_element();
        };
        div()
            .flex()
            .flex_col()
            .gap_8()
            .pt_3()
            .child(self.item(export, theme))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .child(
                        div()
                            .flex()
                            .gap_6()
                            .child(self.price(export, Mode::Sell, theme, cx))
                            .child(self.price(export, Mode::Buy, theme, cx)),
                    )
                    .child(self.actions(theme, cx)),
            )
            .child(self.history(theme, cx))
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap_6()
                    .child(order_book(export, Mode::Sell, theme))
                    .child(order_book(export, Mode::Buy, theme)),
            )
            .into_any_element()
    }

    /// Nothing exported yet, or nowhere to look for it.
    fn empty_state(&self, theme: &Theme, cx: &mut Context<'_, Self>) -> Div {
        let (title, text) = match &self.logs_dir {
            Some(_) => (
                "Waiting for an export",
                "It shows up here the moment the game writes it.",
            ),
            None => (
                "EVE's market exports were not found",
                "Set the Marketlogs folder in Settings. The game creates it with the first export.",
            ),
        };
        let steps = [
            "Open the market in EVE Online and pick an item.",
            "Click Export to File at the bottom of the market window.",
            "The price that beats the best order is on your clipboard.",
            "Paste it into the price field of your order.",
        ];
        div()
            .flex()
            .flex_col()
            .items_center()
            .gap_6()
            .pt_16()
            .child(
                div()
                    .size(px(56.0))
                    .rounded_full()
                    .bg(gpui_kit::white().opacity(0.06))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(icon("icons/file-text.svg", 24.0, theme.muted_foreground)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .text_xl()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(title),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(text),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .children(steps.into_iter().enumerate().map(|(index, step)| {
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(
                                div()
                                    .flex_none()
                                    .w(px(16.0))
                                    .text_color(theme::sell())
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child((index + 1).to_string()),
                            )
                            .child(step)
                    })),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        Button::new("empty-refresh")
                            .label("Look again")
                            .icon(button_icon("icons/refresh-cw.svg"))
                            .small()
                            .on_click(cx.listener(|this, _, _, cx| this.refresh(cx))),
                    )
                    .when(self.logs_dir.is_none(), |d| {
                        d.child(
                            Button::new("empty-settings")
                                .label("Open Settings")
                                .small()
                                .primary()
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.page = Page::Settings;
                                    cx.notify();
                                })),
                        )
                    }),
            )
    }

    /// The item: its icon, name, when it was exported, and what the game says about it.
    fn item(&self, export: &Export, theme: &Theme) -> Div {
        let info = self.details.as_ref().and_then(|d| d.info.as_ref());
        let name = info
            .map(|i| i.name.clone())
            .or_else(|| export.item.clone())
            .unwrap_or_else(|| "Unknown item".into());
        let exported = export
            .exported
            .as_deref()
            .map_or_else(|| "—".to_string(), market::format_stamp);
        let picture = match self.details.as_ref().and_then(|d| d.icon.clone()) {
            Some(image) => img(image).size_full().into_any_element(),
            None => div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(icon(
                    "icons/package.svg",
                    24.0,
                    theme.muted_foreground.opacity(0.5),
                ))
                .into_any_element(),
        };
        div()
            .flex()
            .items_center()
            .gap_4()
            .child(
                div()
                    .flex_none()
                    .size(px(56.0))
                    .rounded(px(12.0))
                    .overflow_hidden()
                    .bg(gpui_kit::white().opacity(0.06))
                    .child(picture),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .min_w_0()
                                    .truncate()
                                    .text_xl()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(name),
                            )
                            .when(self.watcher.is_some(), |d| {
                                d.child(
                                    div()
                                        .flex_none()
                                        .flex()
                                        .items_center()
                                        .gap(px(6.0))
                                        .text_xs()
                                        .text_color(theme.muted_foreground)
                                        .child(div().size(px(6.0)).rounded_full().bg(theme::sell()))
                                        .child("Live"),
                                )
                            }),
                    )
                    .child(caption(format!("Exported {exported}"), theme))
                    .when_some(
                        info.map(|i| i.description.clone())
                            .filter(|d| !d.is_empty()),
                        |d, description| {
                            d.child(
                                div()
                                    .mt_1()
                                    .truncate()
                                    .text_xs()
                                    .text_color(theme.muted_foreground.opacity(0.7))
                                    .child(description),
                            )
                        },
                    ),
            )
    }

    /// One side's price to paste, large. The side on the clipboard is lit and marked by a
    /// bar in its color; clicking the other one switches to it.
    fn price(
        &self,
        export: &Export,
        side: Mode,
        theme: &Theme,
        cx: &mut Context<'_, Self>,
    ) -> impl IntoElement + use<> {
        let active = self.config.mode == side;
        let color = theme::side(side);
        let price = export.price(side);
        let note: SharedString = match (export.best(side), side) {
            (Some(best), Mode::Sell) => format!(
                "One tick under the cheapest sell order, {} ISK",
                ui::isk(best)
            )
            .into(),
            (Some(best), Mode::Buy) => {
                format!("One tick over the highest buy order, {} ISK", ui::isk(best)).into()
            }
            (None, Mode::Sell) => "Nobody is selling".into(),
            (None, Mode::Buy) => "Nobody is buying".into(),
        };
        let status = if !active {
            None
        } else if self.recently_copied() {
            Some(("icons/check.svg", "Copied"))
        } else if self.copied.is_some_and(|(copied, _)| Some(copied) == price) {
            Some(("icons/clipboard-check.svg", "On your clipboard"))
        } else {
            None
        };
        div()
            .id(side.label())
            .flex_1()
            .min_w_0()
            .flex()
            .gap_4()
            .cursor_pointer()
            .when(!active, |d| d.opacity(0.45).hover(|s| s.opacity(0.75)))
            .on_click(cx.listener(move |this, _, _, cx| this.set_mode(side, cx)))
            .child(div().flex_none().w(px(3.0)).rounded_full().bg(if active {
                color
            } else {
                gpui_kit::white().opacity(0.12)
            }))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(color)
                            .child(icon(mode_icon(side), 14.0, color))
                            .child(side.label().to_uppercase())
                            .when_some(status, |d, (path, text)| {
                                d.child(
                                    div()
                                        .ml_1()
                                        .flex()
                                        .items_center()
                                        .gap_1()
                                        .font_weight(FontWeight::NORMAL)
                                        .text_color(theme.muted_foreground)
                                        .child(icon(path, 12.0, theme.muted_foreground))
                                        .child(text),
                                )
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .items_baseline()
                            .gap_2()
                            .mt_1()
                            .child(
                                div()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(px(34.0))
                                    .line_height(px(42.0))
                                    .font_weight(FontWeight::BOLD)
                                    .child(price.map_or_else(|| "—".to_string(), ui::isk)),
                            )
                            .when(price.is_some(), |d| {
                                d.child(
                                    div()
                                        .flex_none()
                                        .text_sm()
                                        .text_color(theme.muted_foreground)
                                        .child("ISK"),
                                )
                            }),
                    )
                    .child(div().truncate().child(caption(note, theme))),
            )
    }

    fn actions(&self, theme: &Theme, cx: &mut Context<'_, Self>) -> Div {
        let copied = self.recently_copied();
        let keys = hotkey::keys(&self.config.hotkey);
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                Button::new("copy")
                    .label(if copied { "Copied" } else { "Copy price" })
                    .icon(button_icon(if copied {
                        "icons/check.svg"
                    } else {
                        "icons/copy.svg"
                    }))
                    .small()
                    .disabled(self.price_now().is_none())
                    .on_click(cx.listener(|this, _, _, cx| this.copy_price(cx))),
            )
            .child(
                Button::new("refresh")
                    .label("Refresh")
                    .icon(button_icon("icons/refresh-cw.svg"))
                    .small()
                    .ghost()
                    .on_click(cx.listener(|this, _, _, cx| this.refresh(cx))),
            )
            .child(div().flex_1())
            .child(
                div()
                    .id("toggle")
                    .flex()
                    .items_center()
                    .gap_2()
                    .h(px(28.0))
                    .px_2()
                    .rounded(px(6.0))
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .cursor_pointer()
                    .hover(|s| s.text_color(theme.foreground))
                    .child("Switch side")
                    .when(!keys.is_empty(), |d| d.child(key_caps(&keys, theme)))
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_mode(cx))),
            )
    }

    /// A month of The Forge's highest and lowest prices.
    fn history(&self, theme: &Theme, cx: &mut Context<'_, Self>) -> Div {
        let days: Rc<[_]> = self
            .details
            .as_ref()
            .map_or_else(|| Rc::from([]), |d| Rc::clone(&d.history));
        let hovered = self.chart_hover.and_then(|i| days.get(i));
        let legend = |color, text: String| {
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(div().size(px(6.0)).rounded_full().bg(color))
                .child(caption(text, theme))
        };
        let (high, low) = match hovered {
            Some(day) => (
                format!("High {}", ui::compact(day.highest)),
                format!("Low {}", ui::compact(day.lowest)),
            ),
            None => ("High".to_string(), "Low".to_string()),
        };
        let header = div()
            .flex()
            .items_center()
            .gap_4()
            .child(heading("Price history"))
            .child(caption("30 days, The Forge", theme))
            .child(div().flex_1())
            .when_some(hovered, |d, day| {
                d.child(div().text_xs().child(ui::short_date(&day.date)))
            })
            .child(legend(theme::sell(), high))
            .child(legend(theme::buy(), low));

        let body = if days.len() > 1 {
            let (low, high) = ui::price_range(&days);
            let labels = [high, f64::midpoint(low, high), low];
            let count = days.len();
            let bounds = Rc::clone(&self.chart_bounds);
            div()
                .flex()
                .gap_3()
                .h(px(130.0))
                .child(
                    div()
                        .id("chart")
                        .flex_1()
                        .min_w_0()
                        .h_full()
                        .on_mouse_move(cx.listener(move |this, event: &MouseMoveEvent, _, cx| {
                            let hover = bounds
                                .get()
                                .and_then(|b| ui::day_at(b, event.position.x, count));
                            if this.chart_hover != hover {
                                this.chart_hover = hover;
                                cx.notify();
                            }
                        }))
                        .on_hover(cx.listener(|this, hovered: &bool, _, cx| {
                            if !hovered && this.chart_hover.take().is_some() {
                                cx.notify();
                            }
                        }))
                        .child(ui::price_chart(
                            days,
                            self.chart_hover,
                            Rc::clone(&self.chart_bounds),
                            theme,
                        )),
                )
                .child(
                    div()
                        .flex_none()
                        .flex()
                        .flex_col()
                        .justify_between()
                        .children(labels.map(|value| caption(ui::compact(value), theme))),
                )
        } else {
            div()
                .h(px(130.0))
                .flex()
                .items_center()
                .justify_center()
                .child(caption(
                    if self.details.is_some() {
                        "No price history for this item"
                    } else {
                        "Loading the price history…"
                    },
                    theme,
                ))
        };
        div().flex().flex_col().gap_3().child(header).child(body)
    }

    // Bulk.

    fn bulk_page(&self, theme: &Theme, cx: &mut Context<'_, Self>) -> AnyElement {
        let empty = self.bulk_input.read(cx).value().trim().is_empty();
        let copied = self.bulk.copied.is_some_and(|at| at.elapsed() < COPIED_FOR);
        let has_prices = self.bulk.rows.iter().any(|row| row.undercut.is_some());
        div()
            .flex()
            .flex_col()
            .gap_4()
            .pt_3()
            .child(page_title(
                "Bulk undercut",
                "Paste a list, one item per line with its quantity after it. Each is priced \
                 one tick under the cheapest sell order in Jita 4-4.",
                theme,
            ))
            .child(Textarea::new(&self.bulk_input).h(px(170.0)))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        Button::new("bulk-run")
                            .label(if self.bulk.loading {
                                "Fetching…"
                            } else {
                                "Fetch and undercut"
                            })
                            .small()
                            .primary()
                            .loading(self.bulk.loading)
                            .disabled(self.bulk.loading || empty)
                            .on_click(cx.listener(|this, _, _, cx| this.run_bulk(cx))),
                    )
                    .child(key_caps(
                        &[
                            if cfg!(target_os = "macos") {
                                "Cmd"
                            } else {
                                "Ctrl"
                            }
                            .to_string(),
                            "Enter".to_string(),
                        ],
                        theme,
                    ))
                    .child(div().flex_1())
                    .when(has_prices, |d| {
                        d.child(
                            Button::new("bulk-copy")
                                .label(if copied { "Copied" } else { "Copy sell orders" })
                                .icon(button_icon(if copied {
                                    "icons/check.svg"
                                } else {
                                    "icons/copy.svg"
                                }))
                                .small()
                                .on_click(cx.listener(|this, _, _, cx| this.copy_bulk(cx))),
                        )
                    }),
            )
            .when_some(self.bulk.problem.clone(), |d, problem| {
                d.child(
                    Self::bar("icons/circle-alert.svg", theme.danger)
                        .child(div().flex_1().min_w_0().child(problem)),
                )
            })
            .when(!self.bulk.rows.is_empty(), |d| {
                d.child(bulk_table(&self.bulk.rows, theme))
            })
            .into_any_element()
    }

    // Settings.

    fn settings_page(&self, theme: &Theme, cx: &mut Context<'_, Self>) -> AnyElement {
        div()
            .flex()
            .flex_col()
            .pt_3()
            .child(self.folder_setting(theme))
            .child(self.shortcut_setting(theme, cx))
            .child(self.about(theme, cx))
            .into_any_element()
    }

    fn folder_setting(&self, theme: &Theme) -> Div {
        let (path, color, status) = match (&self.logs_dir, self.watcher.is_some()) {
            (Some(dir), true) => (
                "icons/circle-check.svg",
                theme::sell(),
                format!("Watching {}", dir.display()),
            ),
            (Some(dir), false) => (
                "icons/circle-alert.svg",
                theme::buy(),
                format!(
                    "Reading {}, but it cannot be watched. Use Refresh after exporting.",
                    dir.display()
                ),
            ),
            (None, _) => (
                "icons/circle-alert.svg",
                theme.danger,
                "No such folder. The game creates it with the first export.".to_string(),
            ),
        };
        setting(
            "Market exports",
            "Where EVE writes Export to File. Found on its own in Documents, OneDrive, the \
             macOS client's folder and Steam's Proton prefix. Leave it empty to look again.",
            theme,
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap_2()
                .child(Input::new(&self.logs_input))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(icon(path, 13.0, color))
                        .child(div().min_w_0().truncate().child(caption(status, theme))),
                ),
        )
    }

    fn shortcut_setting(&self, theme: &Theme, cx: &mut Context<'_, Self>) -> Div {
        let keys = hotkey::keys(&self.config.hotkey);
        let shown = if self.capturing {
            caption("Press the new keys. Esc cancels, Backspace clears.", theme)
                .text_color(theme.primary)
        } else if keys.is_empty() {
            caption("None", theme)
        } else {
            key_caps(&keys, theme)
        };
        let wayland = if cfg!(target_os = "linux") {
            " On Wayland it only fires while an X11 window has focus, which EVE under Proton is."
        } else {
            ""
        };
        setting(
            "Shortcut",
            format!(
                "Switches between selling and buying from anywhere, the game included, and \
                 copies that side's price.{wayland}"
            ),
            theme,
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .items_center()
                .gap_2()
                .child(shown)
                .child(div().flex_1())
                .when(
                    self.config.hotkey != hotkey::DEFAULT && !self.capturing,
                    |d| {
                        d.child(
                            Button::new("hotkey-reset")
                                .label("Reset")
                                .small()
                                .ghost()
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.rebind(hotkey::DEFAULT.to_string());
                                    cx.notify();
                                })),
                        )
                    },
                )
                .child(
                    Button::new("hotkey-change")
                        .label(if self.capturing { "Cancel" } else { "Change" })
                        .small()
                        .on_click(cx.listener(|this, _, window, cx| {
                            if this.capturing {
                                this.capturing = false;
                                cx.notify();
                            } else {
                                this.start_capture(window, cx);
                            }
                        })),
                ),
        )
    }

    fn about(&self, theme: &Theme, cx: &mut Context<'_, Self>) -> Div {
        let status = if self.updater == Updater::Checking {
            Some("Checking…".to_string())
        } else if let Some(available) = &self.update {
            Some(format!("Version {} is out.", available.version))
        } else {
            self.update_note.clone()
        };
        setting(
            "About",
            "A market companion for EVE Online. It updates itself from GitHub releases.",
            theme,
        )
        .border_b_0()
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .min_w_0()
                        .child(
                            div()
                                .text_sm()
                                .child(format!("Marketeer {}", update::VERSION)),
                        )
                        .when_some(status, |d, status| d.child(caption(status, theme))),
                )
                .child(div().flex_1())
                .child(
                    Button::new("about-source")
                        .label("GitHub")
                        .icon(button_icon("icons/external-link.svg"))
                        .small()
                        .ghost()
                        .on_click(|_, _, cx| {
                            cx.open_url(&format!("https://github.com/{}", update::REPO));
                        }),
                )
                .child(
                    Button::new("about-check")
                        .label("Check for updates")
                        .small()
                        .disabled(self.updater != Updater::Idle)
                        .on_click(cx.listener(|this, _, _, cx| this.check_for_update(cx))),
                ),
        )
    }
}

fn mode_icon(mode: Mode) -> &'static str {
    match mode {
        Mode::Sell => "icons/trending-down.svg",
        Mode::Buy => "icons/trending-up.svg",
    }
}

/// The name of a part of a page.
fn heading(text: &'static str) -> Div {
    div()
        .text_sm()
        .font_weight(FontWeight::SEMIBOLD)
        .child(text)
}

/// What a page is, and a line about it.
fn page_title(title: &'static str, text: &'static str, theme: &Theme) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .text_xl()
                .font_weight(FontWeight::SEMIBOLD)
                .child(title),
        )
        .child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(text),
        )
}

/// A setting as a row: what it is and does on the left, its controls (the caller's child)
/// on the right, a hairline under it.
fn setting(title: &'static str, text: impl Into<SharedString>, theme: &Theme) -> Div {
    div()
        .flex()
        .items_start()
        .gap_8()
        .py_6()
        .border_b_1()
        .border_color(theme::hairline())
        .child(
            div()
                .flex_none()
                .w(px(300.0))
                .flex()
                .flex_col()
                .gap_1()
                .child(heading(title))
                .child(caption(text, theme)),
        )
}

/// The best orders of one side, the one to beat on top, each with its volume as a bar
/// behind it.
fn order_book(export: &Export, side: Mode, theme: &Theme) -> Div {
    let color = theme::side(side);
    let (orders, total): (&[Order], usize) = match side {
        Mode::Sell => (&export.sells, export.sell_count),
        Mode::Buy => (&export.buys, export.buy_count),
    };
    let most = orders.iter().map(|o| o.volume).max().unwrap_or(1).max(1);
    div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .flex()
                .items_baseline()
                .gap_2()
                .child(heading(match side {
                    Mode::Sell => "Sell orders",
                    Mode::Buy => "Buy orders",
                }))
                .child(caption(total.to_string(), theme))
                .child(div().flex_1())
                .child(caption("Volume", theme)),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .children(orders.iter().enumerate().map(|(index, order)| {
                    #[expect(
                        clippy::as_conversions,
                        clippy::cast_precision_loss,
                        reason = "a share of the row's width, exactness does not matter"
                    )]
                    let share = (order.volume as f32 / most as f32).clamp(0.02, 1.0);
                    div()
                        .relative()
                        .flex()
                        .items_center()
                        .justify_between()
                        .h(px(26.0))
                        .px_2()
                        .text_xs()
                        .child(
                            div()
                                .absolute()
                                .top_0()
                                .bottom_0()
                                .left_0()
                                .w(relative(share))
                                .rounded(px(4.0))
                                .bg(color.opacity(if index == 0 { 0.16 } else { 0.07 })),
                        )
                        .child(
                            div()
                                .relative()
                                .when(index == 0, |d| {
                                    d.font_weight(FontWeight::SEMIBOLD).text_color(color)
                                })
                                .child(ui::isk(order.price)),
                        )
                        .child(
                            div()
                                .relative()
                                .text_color(theme.muted_foreground)
                                .child(ui::count(order.volume)),
                        )
                })),
        )
        .when(orders.is_empty(), |d| {
            d.child(caption(
                match side {
                    Mode::Sell => "Nobody is selling",
                    Mode::Buy => "Nobody is buying",
                },
                theme,
            ))
        })
}

/// The priced list: item, quantity, Jita's cheapest sell and the price under it.
fn bulk_table(rows: &[Row], theme: &Theme) -> Div {
    let sum = bulk::summary(rows);
    let number = || div().flex_none().w(px(130.0)).flex().justify_end();
    let line = || div().flex().items_center().gap_3().px_2().text_sm();
    let price = |value: Option<f64>| value.map_or_else(|| "—".to_string(), ui::isk);
    div()
        .flex()
        .flex_col()
        .child(
            line()
                .pb_2()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(div().flex_1().child("Item"))
                .child(number().w(px(70.0)).child("Qty"))
                .child(number().child("Lowest sell"))
                .child(number().child("Undercut")),
        )
        .children(rows.iter().map(|row| {
            line()
                .py_2()
                .border_t_1()
                .border_color(theme::hairline())
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .child(div().truncate().child(row.name.clone()))
                        .when_some(row.problem.clone(), |d, problem| {
                            d.child(div().text_xs().text_color(theme.danger).child(problem))
                        }),
                )
                .child(
                    number()
                        .w(px(70.0))
                        .text_color(theme.muted_foreground)
                        .child(ui::count(row.quantity)),
                )
                .child(
                    number()
                        .text_color(theme.muted_foreground)
                        .child(price(row.lowest_sell)),
                )
                .child(
                    number()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(theme::sell())
                        .child(price(row.undercut)),
                )
        }))
        .child(
            line()
                .py_3()
                .border_t_1()
                .border_color(gpui_kit::white().opacity(0.16))
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .gap_2()
                        .text_color(theme.muted_foreground)
                        .child(format!(
                            "{} item{}",
                            sum.items,
                            if sum.items == 1 { "" } else { "s" }
                        ))
                        .when(sum.missing > 0, |d| {
                            d.child(
                                div()
                                    .text_color(theme.danger)
                                    .child(format!("{} without a price", sum.missing)),
                            )
                        }),
                )
                .child(
                    number()
                        .w(px(70.0))
                        .text_color(theme.muted_foreground)
                        .child(ui::count(sum.units)),
                )
                .child(number().text_color(theme.muted_foreground).child("Total"))
                .child(
                    number()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme::sell())
                        .child(ui::isk(sum.total)),
                ),
        )
}
