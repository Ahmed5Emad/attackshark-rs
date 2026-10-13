//! ATTACK SHARK native Linux configurator: eframe/egui frontend over the
//! `attackshark-rs` library. Styled after craft-launcher's design language
//! (square corners, 1px hairlines, mono uppercase labels, dark/light tokens)
//! with our own theme — no brand assets reused.

use std::sync::Arc;

use attackshark_rs::{protocol, Event, Mouse};
use egui::epaint::text::VariationCoords;
use egui::{Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, FontTweak, RichText, Stroke, TextStyle, Ui, Vec2, Visuals};

// ---------------------------------------------------------------- theme ---

#[derive(Clone, Copy)]
struct Tokens {
    dark: bool,
    bg: Color32,
    sunken: Color32,
    ink: Color32,
    strong: Color32,
    muted: Color32,
    faint: Color32,
    line: Color32,
    line_strong: Color32,
    invert_bg: Color32,
    invert_fg: Color32,
    accent: Color32,
    accent_ink: Color32,
    good: Color32,
    warn: Color32,
    bad: Color32,
}

impl Tokens {
    fn dark() -> Self {
        Self {
            dark: true,
            bg: Color32::from_rgb(0x12, 0x13, 0x16),
        sunken: Color32::from_rgb(0x06, 0x06, 0x07),
        ink: Color32::from_rgb(0xf2, 0xf1, 0xee),
        strong: Color32::WHITE,
        muted: Color32::from_rgba_unmultiplied(0xf2, 0xf1, 0xee, 153),
        faint: Color32::from_rgba_unmultiplied(0xf2, 0xf1, 0xee, 97),
        line: Color32::from_rgba_unmultiplied(0xf2, 0xf1, 0xee, 38),
        line_strong: Color32::from_rgba_unmultiplied(0xf2, 0xf1, 0xee, 102),
        invert_bg: Color32::WHITE,
        invert_fg: Color32::from_rgb(0x0b, 0x0b, 0x0c),
        accent: Color32::from_rgb(0x2d, 0x81, 0xff),
        accent_ink: Color32::from_rgb(0x74, 0xaa, 0xff),
        good: Color32::from_rgb(0x3f, 0xd6, 0xb8),
        warn: Color32::from_rgb(0xff, 0xc1, 0x4d),
        bad: Color32::from_rgb(0xff, 0x62, 0x59),
        }
    }
    fn light() -> Self {
        Self {
            dark: false,
            bg: Color32::from_rgb(0xf2, 0xf1, 0xee),
        sunken: Color32::from_rgb(0xe9, 0xe8, 0xe4),
        ink: Color32::from_rgb(0x10, 0x10, 0x14),
        strong: Color32::BLACK,
        muted: Color32::from_rgba_unmultiplied(0x10, 0x10, 0x14, 158),
        faint: Color32::from_rgba_unmultiplied(0x10, 0x10, 0x14, 107),
        line: Color32::from_rgba_unmultiplied(0x10, 0x10, 0x14, 41),
        line_strong: Color32::from_rgba_unmultiplied(0x10, 0x10, 0x14, 102),
        invert_bg: Color32::from_rgb(0x10, 0x10, 0x14),
        invert_fg: Color32::from_rgb(0xf2, 0xf1, 0xee),
        accent: Color32::from_rgb(0x2d, 0x81, 0xff),
        accent_ink: Color32::from_rgb(0x16, 0x59, 0xc4),
        good: Color32::from_rgb(0x0a, 0x75, 0x63),
        warn: Color32::from_rgb(0x8a, 0x58, 0x00),
        bad: Color32::from_rgb(0xb3, 0x26, 0x1e),
        }
    }
    fn get(ctx: &egui::Context) -> Self {
        if ctx.theme() == egui::Theme::Dark { Self::dark() } else { Self::light() }
    }
    fn hairline(&self) -> Stroke {
        Stroke::new(1.0, self.line)
    }
}

const CONTROL_H: f32 = 34.0;
const HUD_SIZE: f32 = 11.0;
const HUD_SPACING: f32 = HUD_SIZE * 0.15;
const BUTTON_SPACING: f32 = HUD_SIZE * 0.12;
/// Craft header / footer heights.
const HEADER_H: f32 = 48.0;
const FOOTER_H: f32 = 52.0;

/// Inter 500.
fn medium() -> FontFamily {
    FontFamily::Name("medium".into())
}
/// Archivo 620 at 118% width: display headings.
fn display() -> FontFamily {
    FontFamily::Name("display".into())
}
/// Geist Mono 500: labels and badges.
fn mono() -> FontFamily {
    FontFamily::Name("mono".into())
}
/// Geist Mono 700: buttons.
fn mono_bold() -> FontFamily {
    FontFamily::Name("mono-bold".into())
}

fn install_fonts(ctx: &egui::Context) {
    const INTER: &[u8] = include_bytes!("../../assets/fonts/Inter-Variable.ttf");
    const ARCHIVO: &[u8] = include_bytes!("../../assets/fonts/Archivo-Variable.ttf");
    const GEIST_MONO: &[u8] = include_bytes!("../../assets/fonts/GeistMono-Variable.ttf");
    let mut fonts = FontDefinitions::default();
    let fallback: Vec<String> = fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    let faces: [(&str, &[u8], Vec<([u8; 4], f32)>, FontFamily); 6] = [
        ("Inter", INTER, vec![(*b"wght", 400.0), (*b"opsz", 14.0)], FontFamily::Proportional),
        ("Inter-Medium", INTER, vec![(*b"wght", 500.0), (*b"opsz", 14.0)], medium()),
        ("Archivo-Display", ARCHIVO, vec![(*b"wght", 620.0), (*b"wdth", 118.0)], display()),
        ("GeistMono", GEIST_MONO, vec![(*b"wght", 400.0)], FontFamily::Monospace),
        ("GeistMono-Medium", GEIST_MONO, vec![(*b"wght", 500.0)], mono()),
        ("GeistMono-Bold", GEIST_MONO, vec![(*b"wght", 700.0)], mono_bold()),
    ];
    for (name, bytes, coords, family) in faces {
        let tweak = FontTweak { coords: VariationCoords::new(coords), ..Default::default() };
        fonts.font_data.insert(name.to_owned(), Arc::new(FontData::from_static(bytes).tweak(tweak)));
        let mut stack = vec![name.to_owned()];
        stack.extend(fallback.iter().cloned());
        fonts.families.insert(family, stack);
    }
    ctx.set_fonts(fonts);
}

fn visuals(t: &Tokens) -> Visuals {
    let mut v = if t.dark { Visuals::dark() } else { Visuals::light() };
    v.panel_fill = t.bg;
    v.window_fill = t.bg;
    v.extreme_bg_color = t.sunken;
    v.text_edit_bg_color = Some(t.sunken);
    v.faint_bg_color = t.sunken;
    v.code_bg_color = t.sunken;
    v.window_stroke = Stroke::new(1.0, t.line_strong);
    v.window_corner_radius = CornerRadius::ZERO;
    v.menu_corner_radius = CornerRadius::ZERO;
    v.override_text_color = Some(t.ink);
    v.hyperlink_color = t.accent_ink;
    v.selection.bg_fill = t.accent;
    v.selection.stroke = Stroke::new(1.0, Color32::WHITE);
    v.warn_fg_color = t.warn;
    v.error_fg_color = t.bad;
    for (w, fill, stroke, fg) in [
        (&mut v.widgets.noninteractive, t.bg, t.line, t.muted),
        (&mut v.widgets.inactive, t.line, t.line, t.ink),
        (&mut v.widgets.hovered, t.sunken, t.line_strong, t.strong),
        (&mut v.widgets.active, t.sunken, t.line_strong, t.strong),
        (&mut v.widgets.open, t.sunken, t.line_strong, t.strong),
    ] {
        w.bg_fill = fill;
        w.weak_bg_fill = fill;
        w.bg_stroke = Stroke::new(1.0, stroke);
        w.corner_radius = CornerRadius::ZERO;
        w.fg_stroke = Stroke::new(1.0, fg);
        w.expansion = 0.0;
    }
    v
}

fn apply_style(ctx: &egui::Context) {
    install_fonts(ctx);
    ctx.set_visuals_of(egui::Theme::Dark, visuals(&Tokens::dark()));
    ctx.set_visuals_of(egui::Theme::Light, visuals(&Tokens::light()));
    ctx.all_styles_mut(|s| {
        s.spacing.item_spacing = Vec2::new(8.0, 6.0);
        s.spacing.icon_spacing = 10.0;
        s.spacing.button_padding = Vec2::new(12.0, 6.0);
        s.spacing.interact_size.y = CONTROL_H;
        s.text_styles = [
            (TextStyle::Heading, FontId::new(24.0, display())),
            (TextStyle::Body, FontId::new(14.0, FontFamily::Proportional)),
            (TextStyle::Button, FontId::new(13.5, medium())),
            (TextStyle::Small, FontId::new(12.0, FontFamily::Proportional)),
            (TextStyle::Monospace, FontId::new(13.0, mono())),
        ]
        .into();
    });
}

// --------------------------------------------------------------- widgets ---
// Craft-launcher language, ported 1:1: mono caps labels, invert-on-active
// nav rows, hairline cells with inverted choice, 20px badges, 8px meters.

fn caps(ui: &Ui, text: &str, size: f32, family: FontFamily, spacing: f32, color: Color32) -> std::sync::Arc<egui::Galley> {
    let format = egui::TextFormat { font_id: FontId::new(size, family), color, extra_letter_spacing: spacing, ..Default::default() };
    ui.painter().layout_job(egui::text::LayoutJob::single_section(text.to_uppercase(), format))
}

fn hud_galley(ui: &Ui, text: &str, color: Color32) -> std::sync::Arc<egui::Galley> {
    caps(ui, text, HUD_SIZE, mono(), HUD_SPACING, color)
}

fn paint_left(ui: &Ui, rect: egui::Rect, galley: std::sync::Arc<egui::Galley>, color: Color32) {
    let pos = egui::pos2(rect.left(), rect.center().y - galley.size().y / 2.0);
    ui.painter().galley_with_override_text_color(pos, galley, color);
}

fn paint_centred(ui: &Ui, rect: egui::Rect, galley: std::sync::Arc<egui::Galley>, color: Color32) {
    ui.painter().galley_with_override_text_color(rect.center() - galley.size() / 2.0, galley, color);
}

fn hud(ui: &mut Ui, text: &str, color: Color32) {
    let galley = hud_galley(ui, text, color);
    let (rect, _) = ui.allocate_exact_size(galley.size(), egui::Sense::hover());
    paint_left(ui, rect, galley, color);
}

fn heading(ui: &mut Ui, text: &str, t: &Tokens) {
    ui.label(RichText::new(text).size(24.0).family(display()).color(t.strong).extra_letter_spacing(-0.02 * 24.0));
}

fn eyebrow(ui: &mut Ui, index: &str, label: &str, t: &Tokens) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        hud(ui, &format!("{index} / "), t.faint);
        hud(ui, label, t.muted);
    });
}

fn badge(ui: &mut Ui, text: &str, color: Color32, t: &Tokens) {
    let galley = caps(ui, text, 10.0, mono(), 1.0, color);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(galley.size().x + 12.0, 20.0), egui::Sense::hover());
    ui.painter().rect(rect, CornerRadius::ZERO, t.sunken, t.hairline(), egui::StrokeKind::Inside);
    paint_centred(ui, rect, galley, color);
}

/// Solid chip with bold mono label (the site's AppTab).
fn tag(ui: &mut Ui, text: &str, fill: Color32, ink: Color32) {
    let galley = caps(ui, text, HUD_SIZE, mono_bold(), HUD_SPACING, ink);
    let (rect, _) = ui.allocate_exact_size(galley.size() + Vec2::new(16.0, 8.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::ZERO, fill);
    paint_centred(ui, rect, galley, ink);
}

/// Full-width nav row: mono label, inverts on hover or when active, accent
/// spine on the active page. This replaces selectable_label (default blue).
fn navlink(ui: &mut Ui, index: &str, label: &str, active: bool, t: &Tokens) -> egui::Response {
    let txt = if index.is_empty() { label.to_uppercase() } else { format!("{index}  {label}") };
    let h = 34.0;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), h), egui::Sense::click());
    let on = active || resp.hovered();
    let col = if on { t.invert_fg } else { t.muted };
    let galley = hud_galley(ui, &txt, col);
    if on {
        ui.painter().rect_filled(rect, CornerRadius::ZERO, t.invert_bg);
    }
    if active {
        ui.painter().rect_filled(egui::Rect::from_min_size(rect.min, Vec2::new(2.0, h)), CornerRadius::ZERO, t.accent);
    }
    paint_left(
        ui,
        egui::Rect::from_min_size(egui::pos2(rect.min.x + 12.0, rect.min.y), Vec2::new(rect.width() - 12.0, h)),
        galley,
        col,
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Mutually exclusive options in a row of hairline cells (the site's share
/// bar); the chosen one is inverted. One outline around the group, rules
/// between neighbours. Returns the clicked index, if any.
fn cells(ui: &mut Ui, opts: &[String], sel: usize, t: &Tokens) -> Option<usize> {
    let mut out = None;
    let mut rects = Vec::with_capacity(opts.len());
    ui.horizontal(|ui| {
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for (i, s) in opts.iter().enumerate() {
            let on = i == sel;
            let g = hud_galley(ui, s, t.ink);
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(g.size().x + 24.0, CONTROL_H), egui::Sense::click());
            let hovered = resp.hovered();
            let (fill, ink) = match (on, hovered) {
                (true, _) => (t.invert_bg, t.invert_fg),
                (false, true) => (t.sunken, t.strong),
                (false, false) => (Color32::TRANSPARENT, t.muted),
            };
            ui.painter().rect_filled(rect, CornerRadius::ZERO, fill);
            paint_centred(ui, rect, g, ink);
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() && !on {
                out = Some(i);
            }
            rects.push(rect);
        }
    });
    });
    if let (Some(first), Some(last)) = (rects.first(), rects.last()) {
        let p = ui.painter();
        p.rect_stroke(first.union(*last), CornerRadius::ZERO, t.hairline(), egui::StrokeKind::Inside);
        for r in rects.iter().skip(1) {
            p.vline(r.left(), first.top()..=first.bottom(), t.hairline());
        }
    }
    out
}

/// Hardware palette swatches: the six LED colors — red, green, blue,
/// yellow, cyan, magenta. No off swatch here: OFF exists as its own mode,
/// and picking off-as-a-color inside static/breathing glitches the LED.
/// The pick flows raw into the 0x05 prefs field; the footer reports the
/// firmware ACK (`...00 05` applied, `...01 05` rejected) honestly.
fn palette(ui: &mut Ui, current: [u8; 3], t: &Tokens) -> Option<[u8; 3]> {
    const PALS: [[u8; 3]; 6] = [
        [255, 0, 0],
        [0, 255, 0],
        [0, 0, 255],
        [255, 255, 0],
        [0, 255, 255],
        [255, 0, 255],
    ];
    let mut out = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        for rgb in PALS {
            let sel = current == rgb;
            let (rect, resp) =
                ui.allocate_exact_size(Vec2::new(34.0, CONTROL_H), egui::Sense::click());
            let p = ui.painter();
            p.rect_filled(rect, CornerRadius::ZERO, Color32::from_rgb(rgb[0], rgb[1], rgb[2]));
            if sel {
                p.rect_stroke(rect, CornerRadius::ZERO, Stroke::new(2.0, t.invert_bg), egui::StrokeKind::Outside);
            }
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() && !sel {
                out = Some(rgb);
            }
        }
    });
    out
}

/// Stage palette swatches. The six real DPI-stage colors in report 0x04 are
/// red/green/blue/yellow/cyan/magenta (vendor capture: 18 writes, ACK
/// `03 24 50 00 04`; stages 7-8 are orange/white 50-DPI fillers). The
/// off/R/G/B-only limit applies to the 0x05 static prefs field, not stages.
fn stage_palette(ui: &mut Ui, current: [u8; 3], t: &Tokens) -> Option<[u8; 3]> {
    const PALS: [[u8; 3]; 6] = [
        [255, 0, 0],
        [0, 255, 0],
        [0, 0, 255],
        [255, 255, 0],
        [0, 255, 255],
        [255, 0, 255],
    ];
    let mut out = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        for rgb in PALS {
            let sel = current == rgb;
            let (rect, resp) =
                ui.allocate_exact_size(Vec2::new(34.0, CONTROL_H), egui::Sense::click());
            let p = ui.painter();
            p.rect_filled(rect, CornerRadius::ZERO, Color32::from_rgb(rgb[0], rgb[1], rgb[2]));
            if sel {
                p.rect_stroke(rect, CornerRadius::ZERO, Stroke::new(2.0, t.invert_bg), egui::StrokeKind::Outside);
            }
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() && !sel {
                out = Some(rgb);
            }
        }
    });
    out
}

/// Slim square progress bar with a caption above it (the site's progress).
fn meter(ui: &mut Ui, frac: Option<f32>, caption: &str, color: Color32, t: &Tokens) {
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, CONTROL_H), egui::Sense::hover());
    let p = ui.painter();
    p.text(egui::pos2(rect.left(), rect.top() + 9.0), egui::Align2::LEFT_CENTER, caption, FontId::new(12.0, medium()), t.muted);
    let bar = egui::Rect::from_min_size(egui::pos2(rect.left(), rect.bottom() - 4.0), Vec2::new(w, 4.0));
    p.rect_filled(bar, CornerRadius::ZERO, t.line);
    if let Some(f) = frac {
        p.rect_filled(
            egui::Rect::from_min_size(bar.min, Vec2::new(bar.width() * f.clamp(0.0, 1.0), bar.height())),
            CornerRadius::ZERO,
            color,
        );
    }
}

/// Hairline card on the flat page surface (the site's cards): same fill as
/// the page, 1px outline, 20px padding.
fn card(ui: &mut Ui, t: &Tokens, add: impl FnOnce(&mut Ui)) {
    egui::Frame::new()
        .fill(t.bg)
        .stroke(t.hairline())
        .inner_margin(egui::Margin::same(20))
        .show(ui, |ui| {
            // Cards always fill their lane — otherwise stacked/narrow lanes
            // leave a dead void beside content-sized rows (pills, checks).
            ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), add);
        });
}

/// Square button with a bold mono upper-case label (the site's button).
/// Primary is a solid inverted block; secondary is outlined and inverts on
/// hover. Returns the response.
fn sbutton(ui: &mut Ui, text: &str, primary: bool, t: &Tokens) -> egui::Response {
    let galley = caps(ui, text, HUD_SIZE, mono_bold(), BUTTON_SPACING, t.ink);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(galley.size().x + 32.0, CONTROL_H), egui::Sense::click());
    let enabled = ui.is_enabled();
    let hovered = resp.hovered() && enabled;
    let (fill, stroke, color) = if primary {
        (t.invert_bg.gamma_multiply(if hovered { 0.8 } else { 1.0 }), Stroke::NONE, t.invert_fg)
    } else if hovered {
        (t.invert_bg, Stroke::NONE, t.invert_fg)
    } else {
        (t.bg, Stroke::new(1.0, t.line_strong), t.ink)
    };
    let fade = |c: Color32| if enabled { c } else { c.gamma_multiply(0.5) };
    ui.painter().rect(rect, CornerRadius::ZERO, fade(fill), Stroke::new(stroke.width, fade(stroke.color)), egui::StrokeKind::Inside);
    paint_centred(ui, rect, galley, fade(color));
    if enabled { resp.on_hover_cursor(egui::CursorIcon::PointingHand) } else { resp }
}

/// Square dropdown in the site language: hairline box, selected value in
/// mono caps, caret drawn by hand; the popup is a stack of navlink rows
/// that invert on hover. Returns the picked index, if any.
fn dropdown(
    ui: &mut Ui,
    id: impl std::hash::Hash + std::fmt::Debug,
    selected: &str,
    options: &[String],
    t: &Tokens,
) -> Option<usize> {
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, CONTROL_H), egui::Sense::click());
    let hovered = resp.hovered();
    let p = ui.painter();
    p.rect(
        rect,
        CornerRadius::ZERO,
        if hovered { t.sunken } else { t.bg },
        t.hairline(),
        egui::StrokeKind::Inside,
    );
    let g = hud_galley(ui, selected, if hovered { t.strong } else { t.ink });
    paint_left(
        ui,
        egui::Rect::from_min_size(
            egui::pos2(rect.left() + 12.0, rect.top()),
            Vec2::new(rect.width() - 36.0, rect.height()),
        ),
        g,
        if hovered { t.strong } else { t.ink },
    );
    let cx = rect.right() - 18.0;
    let cy = rect.center().y;
    p.add(egui::Shape::convex_polygon(
        vec![
            egui::pos2(cx - 5.0, cy - 3.0),
            egui::pos2(cx + 5.0, cy - 3.0),
            egui::pos2(cx, cy + 3.0),
        ],
        t.muted,
        Stroke::NONE,
    ));
    let mut out = None;
    egui::Popup::menu(&resp).id(egui::Id::new(id)).show(|ui| {
        ui.set_min_width(rect.width());
        // Long lists scroll inside the popup instead of blowing up the page.
        let rows = |ui: &mut Ui, out: &mut Option<usize>| {
            for (i, opt) in options.iter().enumerate() {
                if navlink(ui, "", opt, opt == selected, t).clicked() {
                    *out = Some(i);
                    ui.close();
                }
            }
        };
        if options.len() > 7 {
            egui::ScrollArea::vertical().max_height(7.0 * 34.0).show(ui, |ui| {
                rows(ui, &mut out);
            });
        } else {
            rows(ui, &mut out);
        }
    });
    resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    out
}

// ----------------------------------------------------------------- state ---

// One page: core row (device / power / dpi) plus the experimental row below.

struct SharkApp {
    mouse: Option<Mouse>,
    err: String,
    jump: Option<usize>,
    status: String,
    // dpi
    stages: [u16; 6],
    active: u8,
    colors: [[u8; 3]; 6],
    angle_snap: bool,
    ripple: bool,
    // shared prefs
    light: protocol::LightMode,
    led_speed: u8,
    rgb: [u8; 3],
    sleep_min: f32,
    deep_sleep_min: u8,
    debounce_ms: u8,
    polling: u16,
    // buttons: action per slot
    btn_actions: [(usize, u8); 8],
    // macros
    macro_btn: u8,
    macro_times: u8,
    macro_rows: Vec<(String, u32)>,
    // monitor
    monitoring: bool,
    show_hb: bool,
    hb: Option<String>,
    log: Vec<String>,
    battery: Option<u8>,
    last_bt: Option<std::time::Instant>,
    bt_rx: Option<std::sync::mpsc::Receiver<attackshark_rs::bt::BtEvent>>,
}

impl SharkApp {
    fn new() -> Self {
        // Prefer USB (cable/receiver: full read+write); Bluetooth is
        // follow-only. Falls back to BT when no USB node exists.
        let (mouse, err) = match Mouse::open_usb().or_else(|_| Mouse::open()) {
            Ok(m) => (Some(m), String::new()),
            Err(e) => (None, e),
        };
        Self {
            mouse,
            err,
            jump: None,
            status: String::from("ready"),
            stages: [800, 1600, 2400, 3200, 8000, 25000],
            active: 2,
            colors: [[0xff, 0, 0], [0, 0xff, 0], [0, 0, 0xff], [0xff, 0xff, 0], [0, 0xff, 0xff], [0xff, 0, 0xff]],
            angle_snap: true,
            ripple: false,
            light: protocol::LightMode::Static,
            led_speed: 3,
            rgb: [0, 255, 0],
            sleep_min: 5.0,
            deep_sleep_min: 10,
            debounce_ms: 8,
            polling: 1000,
            btn_actions: [
                (protocol::buttons::LEFT, protocol::buttons::LEFT_CLICK),
                (protocol::buttons::RIGHT, protocol::buttons::RIGHT_CLICK),
                (protocol::buttons::MIDDLE, protocol::buttons::MIDDLE_CLICK),
                (protocol::buttons::DPI, protocol::buttons::DPI_CYCLE),
                (protocol::buttons::FORWARD, protocol::buttons::ACT_FORWARD),
                (protocol::buttons::BACKWARD, protocol::buttons::ACT_BACKWARD),
                (protocol::buttons::SCROLL_UP, protocol::buttons::SCROLL_UP_ACT),
                (protocol::buttons::SCROLL_DOWN, protocol::buttons::SCROLL_DOWN_ACT),
            ],
            macro_btn: protocol::macros::BTN_EXTRA4,
            macro_times: 1,
            macro_rows: vec![("a".into(), 100), ("b".into(), 100)],
            monitoring: false,
            show_hb: false,
            hb: None,
            battery: None,
            last_bt: None,
            bt_rx: None,
            log: Vec::new(),
        }
    }

    fn prefs(&self) -> Option<protocol::Prefs> {
        protocol::build_prefs(protocol::Prefs {
            light: self.light,
            led_speed: self.led_speed,
            rgb: self.rgb,
            sleep_min: self.sleep_min,
            deep_sleep_min: self.deep_sleep_min,
            debounce_ms: self.debounce_ms,
        })
        .map(|_| protocol::Prefs {
            light: self.light,
            led_speed: self.led_speed,
            rgb: self.rgb,
            sleep_min: self.sleep_min,
            deep_sleep_min: self.deep_sleep_min,
            debounce_ms: self.debounce_ms,
        })
    }

    fn note(&mut self, ok: Result<Vec<u8>, String>, what: &str) {
        match ok {
            Ok(ack) => self.status = format!("{what}: ACK [{}]", attackshark_rs::hex(&ack)),
            Err(e) => self.status = format!("{what} FAILED: {e}"),
        }
    }
}

// ------------------------------------------------------------------- ui ---

impl eframe::App for SharkApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let t = Tokens::get(&ctx);
        if self.monitoring {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        } else {
            ctx.request_repaint_after(std::time::Duration::from_millis(2000));
        }
        if let Some(m) = &self.mouse {
            let bt = m.transport() == "Bluetooth";
            while let Ok(ev) = m.read_event(0) {
                match &ev {
                    Event::Battery(pct) => self.battery = Some(*pct),
                    Event::Heartbeat(b) => {
                        // Live follow on cable/2.4G: 03 24 10 <stage 1-6> 00
                        // announces the stage, 03 24 41 <mode> <batt%>
                        // carries charge (proven wired 2026-10-12).
                        if b.len() == 5 && b[0] == 0x03 && b[1] == 0x24 {
                            if b[2] == 0x10 && (1..=6).contains(&b[3]) {
                                self.active = b[3];
                            } else if b[2] == 0x41 && b[4] <= 100 {
                                self.battery = Some(b[4]);
                            }
                        }
                        self.hb = Some(format!("{ev:?}"));
                        if self.monitoring && self.show_hb {
                            self.log.push(format!("{ev:?}"));
                            if self.log.len() > 200 {
                                self.log.remove(0);
                            }
                        }
                    }
                    _ => {
                        if self.monitoring {
                            self.log.push(format!("{ev:?}"));
                            if self.log.len() > 200 {
                                self.log.remove(0);
                            }
                        }
                    }
                }
            }
            // Bluetooth carries no battery on the HID channel — read it from
            // BlueZ Battery Service instead, at most every 5s. Stage
            // announcements arrive over the BLE notify channel via a
            // background thread (see src/bt.rs); drain it here.
            if bt {
                let stale = self
                    .last_bt
                    .map(|t0| t0.elapsed().as_secs() >= 5)
                    .unwrap_or(true);
                if stale {
                    if let Some(pct) = attackshark_rs::bt_battery_pct() {
                        self.battery = Some(pct);
                    }
                    self.last_bt = Some(std::time::Instant::now());
                }
                if self.bt_rx.is_none() {
                    let (tx, rx) = std::sync::mpsc::channel();
                    self.bt_rx = Some(rx);
                    std::thread::spawn(move || attackshark_rs::bt::monitor(tx));
                }
            }
            if let Some(rx) = &self.bt_rx {
                while let Ok(ev) = rx.try_recv() {
                    match ev {
                        attackshark_rs::bt::BtEvent::Battery(pct) => {
                            self.battery = Some(pct);
                        }
                        attackshark_rs::bt::BtEvent::Stage(s) => {
                            self.active = s;
                        }
                    }
                }
            }
        }

        // header: 48px bar — wordmark cell, tag, subtitle; status badges right.
        egui::Panel::top("header")
            .frame(egui::Frame::new().fill(t.bg))
            .exact_size(HEADER_H)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    let (cell, _) = ui.allocate_exact_size(Vec2::new(232.0, HEADER_H), egui::Sense::hover());
                    ui.painter().rect_filled(cell, CornerRadius::ZERO, t.bg);
                    let mut job = egui::text::LayoutJob::default();
                    job.append("ATTACK ", 0.0, egui::TextFormat { font_id: FontId::new(17.0, display()), color: t.strong, ..Default::default() });
                    job.append("SHARK", 0.0, egui::TextFormat { font_id: FontId::new(17.0, display()), color: t.accent_ink, ..Default::default() });
                    let g = ui.painter().layout_job(job);
                    ui.painter().galley(egui::pos2(cell.left() + 20.0, cell.center().y - g.size().y / 2.0), g, t.strong);
                    ui.painter().vline(cell.right(), cell.y_range(), t.hairline());
                    ui.add_space(12.0);
                    tag(ui, "V6", t.invert_bg, t.invert_fg);
                    ui.add_space(8.0);
                    hud(ui, "native linux configurator", t.faint);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(20.0);
                        if self.mouse.is_some() {
                            match self.battery {
                                Some(p) => {
                                    let c = if p > 50 { t.good } else if p > 20 { t.warn } else { t.bad };
                                    badge(ui, &format!("battery {p}%"), c, &t);
                                }
                                None => badge(ui, "battery —", t.faint, &t),
                            }
                            ui.add_space(6.0);
                            badge(ui, "connected", t.good, &t);
                        } else {
                            badge(ui, "no device", t.bad, &t);
                        }
                    });
                });
            });

        // footer: bar — status left, reset + reconnect + heartbeat + error
        // right, everything on the vertical centre.
        egui::Panel::bottom("footer")
            .frame(egui::Frame::new().fill(t.bg).inner_margin(egui::Margin::symmetric(20, 0)))
            .exact_size(FOOTER_H)
            .show(ui, |ui| {
                let h = ui.available_height();
                ui.horizontal_centered(|ui| {
                    ui.set_min_height(h);
                    ui.spacing_mut().item_spacing.x = 0.0;
                    hud(ui, &self.status, t.muted);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if sbutton(ui, "reset", true, &t).clicked() {
                            self.do_reset();
                        }
                        ui.add_space(8.0);
                        if sbutton(ui, "reconnect", false, &t).clicked() {
                            self.reconnect();
                        }
                        ui.add_space(12.0);
                        if !self.err.is_empty() {
                            hud(ui, &self.err, t.bad);
                            ui.add_space(12.0);
                        }
                        hud(ui, &self.hb.clone().unwrap_or_else(|| "—".into()), t.faint);
                    });
                });
            });

        // centre: one page — core row on top, experimental row alone below.
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(t.bg).inner_margin(egui::Margin { left: 20, right: 20, top: 18, bottom: 12 }))
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    // bento row A: core — device | power + lighting | dpi.
                    // Narrow windows stack the cards instead of squeezing them.
                    eyebrow(ui, "co", "core", &t);
                    ui.add_space(8.0);
                    if ui.available_width() < 760.0 {
                        self.page_device(ui, &t);
                        ui.add_space(12.0);
                        self.page_power(ui, &t);
                        ui.add_space(12.0);
                        self.page_lighting(ui, &t);
                        ui.add_space(12.0);
                        self.page_dpi(ui, &t);
                    } else {
                        ui.columns(3, |cols| {
                            self.page_device(&mut cols[0], &t);
                            self.page_power(&mut cols[1], &t);
                            cols[1].add_space(12.0);
                            self.page_lighting(&mut cols[1], &t);
                            self.page_dpi(&mut cols[2], &t);
                        });
                    }
                    ui.add_space(12.0);
                    // bento row B: experimental — buttons + macros together, alone.
                    eyebrow(ui, "ex", "experimental", &t);
                    ui.add_space(8.0);
                    if ui.available_width() < 560.0 {
                        self.page_buttons(ui, &t);
                        ui.add_space(12.0);
                        self.page_macros(ui, &t);
                    } else {
                        ui.columns(2, |cols| {
                            self.page_buttons(&mut cols[0], &t);
                            self.page_macros(&mut cols[1], &t);
                        });
                    }
                });
            });
    }
}

impl SharkApp {
    fn page_device(&mut self, ui: &mut Ui, t: &Tokens) {
        card(ui, t, |ui| {
            hud(ui, "link status", t.muted);
            ui.add_space(4.0);
            if let Some(m) = &self.mouse {
                egui::Grid::new("devstat").num_columns(2).spacing([16.0, 6.0]).show(ui, |ui| {
                    hud(ui, "link", t.faint);
                    ui.label(format!("{} · {} {}", m.transport(), m.manufacturer().unwrap_or_default(), m.product().unwrap_or_default()));
                    ui.end_row();
                    hud(ui, "channel", t.faint);
                    ui.monospace(m.channel());
                    ui.end_row();
                    hud(ui, "feature a0", t.faint);
                    if m.transport() == "Bluetooth" {
                        hud(ui, "usb receiver only", t.faint);
                    } else {
                        match m.feature_a0() {
                            Ok(b) => { ui.monospace(format!("[{}]", attackshark_rs::hex(&b))); }
                            Err(e) => { ui.label(RichText::new(e).color(t.bad)); }
                        }
                    }
                    ui.end_row();
                    hud(ui, "heartbeat", t.faint);
                    ui.monospace(self.hb.clone().unwrap_or_else(|| "—".into()));
                    ui.end_row();
                });
            } else {
                ui.label(RichText::new("Receiver not found. Plug it in, then Reconnect.").color(t.warn));
            }
        });
        ui.add_space(12.0);
        card(ui, t, |ui| {
            hud(ui, "battery", t.muted);
            ui.add_space(2.0);
            match self.battery {
                Some(p) => {
                    let c = if p > 50 { t.good } else if p > 20 { t.warn } else { t.bad };
                    meter(ui, Some(p as f32 / 100.0), &format!("{p}% — reported by the mouse"), c, t)
                }
                None => meter(ui, None, "no reading yet", t.faint, t),
            }
            ui.add_space(2.0);
            hud(ui, "charge rides inside the heartbeat — updates live while connected", t.faint);
        });
        ui.add_space(12.0);
        card(ui, t, |ui| {
            hud(ui, "live events", t.muted);
            ui.add_space(4.0);
            ui.checkbox(&mut self.monitoring, "monitor device events");
            ui.checkbox(&mut self.show_hb, "include heartbeat bytes");
            if self.monitoring {
                ui.add_space(6.0);
                egui::Frame::new().fill(t.bg).stroke(t.hairline()).inner_margin(8.0).show(ui, |ui| {
                    for line in self.log.iter().rev().take(8) {
                        ui.monospace(line);
                    }
                    if self.log.is_empty() {
                        hud(ui, "press the DPI button or plug the charger to learn events", t.faint);
                    }
                });
            }
        });
    }

    fn page_dpi(&mut self, ui: &mut Ui, t: &Tokens) {
        card(ui, t, |ui| {
            ui.horizontal(|ui| {
                hud(ui, "dpi stages", t.muted);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if sbutton(ui, "apply", true, t).clicked() {
                        self.apply_dpi();
                    }
                });
            });
            ui.add_space(4.0);
            let names: Vec<String> = (1..=6).map(|i| format!("S{i}")).collect();
            if let Some(i) = cells(ui, &names, (self.active as usize).saturating_sub(1).min(5), t) {
                self.active = i as u8 + 1;
            }
            ui.add_space(6.0);
            ui.columns(2, |uis| {
            for col in 0..2 {
                uis[col].vertical(|ui| {
                for k in 0..3 {
                let i = col + k * 2;
                let on = self.active == i as u8 + 1;
                if k > 0 { ui.add_space(12.0); }
                ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    let (sp, _) = ui.allocate_exact_size(Vec2::new(2.0, CONTROL_H), egui::Sense::hover());
                    ui.painter().rect_filled(sp, CornerRadius::ZERO, if on { t.accent } else { t.bg });
                    tag(ui, &format!("S{}", i + 1), if on { t.invert_bg } else { t.sunken }, if on { t.invert_fg } else { t.muted });
                    let dot = Color32::from_rgb(self.colors[i][0], self.colors[i][1], self.colors[i][2]);
                    let (r, resp) = ui.allocate_exact_size(Vec2::new(12.0, 12.0), egui::Sense::click());
                    ui.painter().rect_filled(r, CornerRadius::ZERO, dot);
                    egui::Popup::menu(&resp).show(|ui| {
                        egui::Grid::new(("stagepop", i))
                            .num_columns(3)
                            .spacing([6.0, 6.0])
                            .show(ui, |ui| {
                                for (n, (_, rgb)) in Self::STAGE_COLORS.iter().enumerate() {
                                    let sel = *rgb == self.colors[i];
                                    let (sr, sresp) = ui.allocate_exact_size(Vec2::new(28.0, 28.0), egui::Sense::click());
                                    ui.painter().rect_filled(sr, CornerRadius::ZERO, Color32::from_rgb(rgb[0], rgb[1], rgb[2]));
                                    if sel {
                                        ui.painter().rect_stroke(sr, CornerRadius::ZERO, Stroke::new(2.0, t.invert_bg), egui::StrokeKind::Outside);
                                    }
                                    if sresp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                        self.colors[i] = *rgb;
                                        ui.close();
                                    }
                                    if n % 3 == 2 {
                                        ui.end_row();
                                    }
                                }
                            });
                    });
                    resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("click: pick LED color");
                    ui.label(RichText::new(format!("{}", self.stages[i])).size(22.0).family(display()).strong().color(if on { t.strong } else { t.ink }));
                    hud(ui, "dpi", t.faint);
                    hud(ui, Self::stage_name(self.colors[i]), dot);
                });
                let mut v = self.stages[i] as f32;
                if ui.add_sized(Vec2::new(ui.available_width(), CONTROL_H), egui::Slider::new(&mut v, 50.0..=25000.0).step_by(50.0).show_value(false).trailing_fill(true)).changed() {
                    self.stages[i] = (v as u16).clamp(50, 25000);
                }
                });
                }
                });
            }
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.angle_snap, "angle snap");
                ui.checkbox(&mut self.ripple, "ripple control");
            });
        });
    }

    fn page_lighting(&mut self, ui: &mut Ui, t: &Tokens) {
        card(ui, t, |ui| {
            ui.horizontal(|ui| {
                hud(ui, "light mode", t.muted);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if sbutton(ui, "apply", true, t).clicked() {
                        self.apply_lighting();
                    }
                });
            });
            ui.add_space(4.0);
            let modes = [
                (protocol::LightMode::Off, "off"),
                (protocol::LightMode::Static, "static"),
                (protocol::LightMode::Breathing, "breathing"),
                (protocol::LightMode::Neon, "neon"),
                (protocol::LightMode::ColorBreathing, "color cycle"),
                (protocol::LightMode::BreathingDpi, "breathing + dpi"),
            ];
            // 3-col x 2-row grid.
            ui.columns(3, |uis| {
                for col in 0..3 {
                    uis[col].vertical(|ui| {
                        for k in 0..2 {
                            let i = k * 3 + col;
                            if k > 0 {
                                ui.add_space(8.0);
                            }
                            let (m, label) = &modes[i];
                            if navlink(ui, "", label, self.light == *m, t).clicked() {
                                self.light = *m;
                            }
                        }
                    });
                }
            });
            // dynamic: color only for modes that take a fixed color,
            // speed only for modes that animate.
            let needs_color = matches!(
                self.light,
                protocol::LightMode::Static | protocol::LightMode::Breathing
            );
            let needs_speed = matches!(
                self.light,
                protocol::LightMode::Breathing
                    | protocol::LightMode::Neon
                    | protocol::LightMode::ColorBreathing
                    | protocol::LightMode::BreathingDpi
            );
            if needs_color {
                ui.add_space(6.0);
                hud(ui, "color", t.muted);
                ui.add_space(4.0);
                if let Some(rgb) = palette(ui, self.rgb, t) {
                    self.rgb = rgb;
                }
            }
            if needs_speed {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    hud(ui, "speed", t.muted);
                    ui.add(egui::Slider::new(&mut self.led_speed, 1..=5));
                });
            }
        });
    }

    fn page_power(&mut self, ui: &mut Ui, t: &Tokens) {
        card(ui, t, |ui| {
            hud(ui, "polling rate", t.muted);
            ui.add_space(4.0);
            let hz = [125u16, 250, 500, 1000];
            let labels: Vec<String> = hz.iter().map(|h| format!("{h} Hz")).collect();
            let cur = hz.iter().position(|h| *h == self.polling).unwrap_or(3);
            if let Some(i) = cells(ui, &labels, cur, t) {
                self.polling = hz[i];
                if let Some(m) = &self.mouse {
                    self.note(m.set_polling(hz[i]), "polling");
                }
            }
            ui.add_space(4.0);
            hud(ui, "proven live on V6 hardware", t.good);
        });
        ui.add_space(12.0);
        card(ui, t, |ui| {
            ui.horizontal(|ui| {
                hud(ui, "timers", t.muted);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if sbutton(ui, "apply", true, t).clicked() {
                        self.apply_power();
                    }
                });
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                hud(ui, "sleep", t.muted);
                ui.add(egui::Slider::new(&mut self.sleep_min, 0.5..=30.0).suffix(" min"));
            });
            ui.horizontal(|ui| {
                hud(ui, "deep sleep", t.muted);
                ui.add(egui::Slider::new(&mut self.deep_sleep_min, 1..=60).suffix(" min"));
            });
            ui.horizontal(|ui| {
                hud(ui, "debounce", t.muted);
                let mut d = self.debounce_ms as f32;
                if ui.add(egui::Slider::new(&mut d, 4.0..=50.0).step_by(2.0).suffix(" ms")).changed() {
                    self.debounce_ms = d as u8;
                }
            });

        });
    }

    /// Firmware stage-LED palette, proven from the vendor capture (18 writes,
    /// all ACK 00 04). Drives the per-stage color dropdowns.
    const STAGE_COLORS: [(&'static str, [u8; 3]); 6] = [
        ("red", [0xff, 0, 0]),
        ("green", [0, 0xff, 0]),
        ("blue", [0, 0, 0xff]),
        ("yellow", [0xff, 0xff, 0]),
        ("cyan", [0, 0xff, 0xff]),
        ("magenta", [0xff, 0, 0xff]),
    ];

    const BTN_NAMES: [(&'static str, usize); 8] = [
        ("left", protocol::buttons::LEFT),
        ("right", protocol::buttons::RIGHT),
        ("middle", protocol::buttons::MIDDLE),
        ("dpi", protocol::buttons::DPI),
        ("forward", protocol::buttons::FORWARD),
        ("backward", protocol::buttons::BACKWARD),
        ("wheel up", protocol::buttons::SCROLL_UP),
        ("wheel down", protocol::buttons::SCROLL_DOWN),
    ];

    const ACTIONS: [(&'static str, u8); 12] = [
        ("left click", protocol::buttons::LEFT_CLICK),
        ("right click", protocol::buttons::RIGHT_CLICK),
        ("middle click", protocol::buttons::MIDDLE_CLICK),
        ("back", protocol::buttons::ACT_BACKWARD),
        ("forward", protocol::buttons::ACT_FORWARD),
        ("double click", protocol::buttons::DOUBLE_CLICK),
        ("dpi cycle", protocol::buttons::DPI_CYCLE),
        ("dpi up", protocol::buttons::DPI_UP),
        ("dpi down", protocol::buttons::DPI_DOWN),
        ("play/pause", protocol::buttons::PLAY_PAUSE),
        ("mute", protocol::buttons::MUTE),
        ("disabled", protocol::buttons::DISABLE),
    ];

    fn page_buttons(&mut self, ui: &mut Ui, t: &Tokens) {
        badge(ui, "experimental — X11 layout, test one button first", t.warn, t);
        ui.add_space(8.0);
        card(ui, t, |ui| {
            hud(ui, "map", t.muted);
            ui.add_space(4.0);
            for i in 0..8 {
                let (name, _) = Self::BTN_NAMES[i];
                ui.horizontal(|ui| {
                    hud(ui, name, t.muted);
                    let options: Vec<String> =
                        Self::ACTIONS.iter().map(|(label, _)| label.to_string()).collect();
                    if let Some(pick) =
                        dropdown(ui, format!("btn{i}"), Self::action_name(self.btn_actions[i].1), &options, t)
                    {
                        self.btn_actions[i].1 = Self::ACTIONS[pick].1;
                    }
                });
            }
        });
        ui.add_space(8.0);
        if sbutton(ui, "apply buttons to mouse", true, t).clicked() {
            self.apply_buttons();
        }
    }

    fn stage_name(rgb: [u8; 3]) -> &'static str {
        Self::STAGE_COLORS.iter().find(|(_, c)| *c == rgb).map(|(n, _)| *n).unwrap_or("custom")
    }

    fn action_name(act: u8) -> &'static str {
        Self::ACTIONS.iter().find(|(_, a)| *a == act).map(|(n, _)| *n).unwrap_or("?")
    }

    fn page_macros(&mut self, ui: &mut Ui, t: &Tokens) {
        badge(ui, "experimental — needs live test before shipping", t.warn, t);
        ui.add_space(8.0);
        card(ui, t, |ui| {
            hud(ui, "target", t.muted);
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                hud(ui, "button", t.muted);
                let mbtns = [
                    ("left", protocol::macros::BTN_LEFT),
                    ("right", protocol::macros::BTN_RIGHT),
                    ("middle", protocol::macros::BTN_MIDDLE),
                    ("extra4", protocol::macros::BTN_EXTRA4),
                    ("extra5", protocol::macros::BTN_EXTRA5),
                ];
                let options: Vec<String> =
                    mbtns.iter().map(|(label, b)| format!("{label} (id {b})")).collect();
                let sel = mbtns
                    .iter()
                    .find(|(_, b)| *b == self.macro_btn)
                    .map(|(label, _)| label.to_string())
                    .unwrap_or_else(|| format!("id {}", self.macro_btn));
                if let Some(pick) = dropdown(ui, "mbtn", &sel, &options, t) {
                    self.macro_btn = mbtns[pick].1;
                }
                hud(ui, "repeat", t.muted);
                ui.add(egui::DragValue::new(&mut self.macro_times).range(1..=255));
            });
        });
        ui.add_space(12.0);
        card(ui, t, |ui| {
            hud(ui, "events — key text + delay ms (max 47)", t.muted);
            ui.add_space(4.0);
            let mut drop = None;
            for (i, (key, ms)) in self.macro_rows.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(key);
                    ui.add(egui::DragValue::new(ms).range(0..=5000).suffix(" ms"));
                    if ui.small_button("x").clicked() {
                        drop = Some(i);
                    }
                });
            }
            if let Some(i) = drop {
                self.macro_rows.remove(i);
            }
            ui.add_space(4.0);
            if sbutton(ui, "add event", false, t).clicked() && self.macro_rows.len() < 47 {
                self.macro_rows.push(("a".into(), 100));
            }
        });
        ui.add_space(8.0);
        if sbutton(ui, "send macro to mouse", true, t).clicked() {
            self.send_macro();
        }
    }

    /// Vendor Apply always opens with the 0x0c commit; the mouse ignores some
    /// pages (proven live: 0x05 prefs) without it. Failures here are ignored
    /// on purpose — the page send still runs (proven: 0x04 DPI ACKs alone).
    fn commit(&mut self) {
        if let Some(m) = &self.mouse {
            let _ = m.send_commit();
        }
    }

    fn reconnect(&mut self) {
        match Mouse::open_usb().or_else(|_| Mouse::open()) {
            Ok(m) => {
                self.mouse = Some(m);
                self.err.clear();
                self.status = "reconnected".into();
            }
            Err(e) => self.err = e,
        }
    }

    /// Factory reset: replays the vendor default Apply on the mouse, then
    /// mirrors the same defaults back into the UI state so the next per-page
    /// Apply sends factory values, not the stale pre-reset ones.
    fn do_reset(&mut self) {
        let r = match self.mouse.as_ref() {
            Some(m) => m.reset_factory(),
            None => Err("no device".into()),
        };
        match r {
            Ok(()) => {
                self.stages = [800, 1600, 2400, 3200, 8000, 25000];
                self.active = 2;
                self.colors = [[0xff, 0, 0], [0, 0xff, 0], [0, 0, 0xff], [0xff, 0xff, 0], [0, 0xff, 0xff], [0xff, 0, 0xff]];
                self.angle_snap = true;
                self.ripple = false;
                self.light = protocol::LightMode::Off;
                self.led_speed = 3;
                self.rgb = [0, 0xff, 0];
                self.sleep_min = 0.5;
                self.deep_sleep_min = 10;
                self.debounce_ms = 8;
                self.polling = 1000;
                self.btn_actions = [
                    (protocol::buttons::LEFT, protocol::buttons::LEFT_CLICK),
                    (protocol::buttons::RIGHT, protocol::buttons::RIGHT_CLICK),
                    (protocol::buttons::MIDDLE, protocol::buttons::MIDDLE_CLICK),
                    (protocol::buttons::DPI, protocol::buttons::DPI_CYCLE),
                    (protocol::buttons::FORWARD, protocol::buttons::ACT_FORWARD),
                    (protocol::buttons::BACKWARD, protocol::buttons::ACT_BACKWARD),
                    (protocol::buttons::SCROLL_UP, protocol::buttons::SCROLL_UP_ACT),
                    (protocol::buttons::SCROLL_DOWN, protocol::buttons::SCROLL_DOWN_ACT),
                ];
                self.status = "reset: factory defaults applied".into();
            }
            Err(e) => self.status = format!("reset failed: {e}"),
        }
    }

    fn apply_dpi(&mut self) {
        self.commit();
        if let Some(m) = &self.mouse {
            let stages8 = [self.stages[0], self.stages[1], self.stages[2], self.stages[3], self.stages[4], self.stages[5], 50, 50];
            let colors8 = [self.colors[0], self.colors[1], self.colors[2], self.colors[3], self.colors[4], self.colors[5], [0, 0, 0], [0, 0, 0]];
            let r = m.set_dpi_full(stages8, self.active, self.angle_snap, self.ripple, 0x20, colors8);
            self.note(r, "dpi");
        } else {
            self.status = "no device".into();
        }
    }

    fn apply_lighting(&mut self) {
        self.commit();
        match (self.mouse.as_ref(), self.prefs()) {
            (Some(m), Some(p)) => self.note(m.set_prefs(p), "lighting"),
            _ => self.status = "no device or bad values".into(),
        }
    }

    fn apply_power(&mut self) {
        self.commit();
        match (self.mouse.as_ref(), self.prefs()) {
            (Some(m), Some(p)) => self.note(m.set_prefs(p), "power prefs"),
            _ => self.status = "no device or bad values".into(),
        }
    }

    fn apply_buttons(&mut self) {
        self.commit();
        if let Some(m) = &self.mouse {
            let mut map = protocol::buttons::blank();
            for (off, act) in self.btn_actions {
                map = protocol::buttons::set_slot(map, off, act, 0, 0);
            }
            self.note(m.set_buttons_raw(&map), "buttons");
        } else {
            self.status = "no device".into();
        }
    }

    fn send_macro(&mut self) {
        self.commit();
        if let Some(m) = &self.mouse {
            let events: Vec<(u8, u8)> = self
                .macro_rows
                .iter()
                .map(|(k, ms)| {
                    let hid = k.bytes().next().unwrap_or(b'a');
                    (protocol::macros::delay_byte(*ms), hid)
                })
                .collect();
            let pkts = protocol::macros::build_packets(self.macro_btn, protocol::macros::PLAY_N_TIMES, self.macro_times, &events);
            let mut map = protocol::buttons::blank();
            map = protocol::buttons::set_slot(map, protocol::buttons::DPI, protocol::buttons::CUSTOM_MACRO, 0, 0);
            match m.set_buttons_raw(&map).and_then(|_| m.send_macro_packets(&pkts).map(|_| Vec::<u8>::new())) {
                Ok(_) => self.status = "macro sent (verify on device)".into(),
                Err(e) => self.status = format!("macro FAILED: {e}"),
            }
        } else {
            self.status = "no device".into();
        }
    }
}

fn main() {
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1120.0, 720.0]).with_title("ATTACK SHARK"),
        ..Default::default()
    };
    if let Err(e) = eframe::run_native("ATTACK SHARK", opts, Box::new(|cc| {
        apply_style(&cc.egui_ctx);
        Ok(Box::new(SharkApp::new()))
    })) {
        eprintln!("UI_FAIL: {e}");
    }
}
