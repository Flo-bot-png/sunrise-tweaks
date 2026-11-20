//! Das Klick-Menü: eine Taste (Standard: rechte Shift-Taste) öffnet über dem Spiel ein Menü
//! mit einem Fenster pro Kategorie.
//!
//! - **Linksklick** auf ein Modul schaltet es an/aus.
//! - **Rechtsklick** auf ein Modul klappt seine Einstellungen auf/zu.
//! - Einstellungen: Klick schaltet/wechselt; Zahlen: links = weniger, rechts = mehr (oder Mausrad).
//! - **Taste:** anklicken, dann die neue Taste drücken (Esc = abbrechen, Entf/Rücktaste = keine).
//! - Fenster lassen sich an der Titelleiste verschieben. Escape schließt das Menü.

use mod_api::*;

use crate::{MOD_ID, State, UNBOUND};

pub const KEY_GUI: &str = "open_gui";

const HEADER_H: f32 = 16.0;
const ROW_H: f32 = 13.0;
const OPT_H: f32 = 12.0;
const SLIDER_H: f32 = 6.0;
const GAP: f32 = 4.0;
const MARGIN: f32 = 4.0;

// Farben (ARGB)
const BG: u32 = 0xE012_1218;
const HEADER: u32 = 0xFF6C_3FD1;
const ROW_HOVER: u32 = 0x40FF_FFFF;
const OPT_BG: u32 = 0xE00B_0B10;
const TEXT: u32 = 0xFFE6_E6E6;
const TEXT_DIM: u32 = 0xFF8A_8A93;
const ON: u32 = 0xFFB0_8CFF;
const LISTEN: u32 = 0xFFFF_D24A;
const SCREEN_DIM: u32 = 0x6000_0000;
const TRACK: u32 = 0xFF3A_3A44;

/// Wie ein Modul an- und ausgeht.
pub enum Enable {
    /// Ein An/Aus-Wert aus der Konfiguration, mit Standardwert.
    Setting(&'static str, bool),
    /// Der Autoclicker (läuft nur, solange das Spiel offen ist).
    Clicker,
}

/// Eine Einstellung unter einem Modul.
pub enum Opt {
    Toggle { key: &'static str, label: &'static str, default: bool },
    Choice { key: &'static str, label: &'static str, options: &'static [&'static str] },
    Range { key: &'static str, label: &'static str, min: f64, max: f64, step: f64, default: f64 },
    /// Eine Taste aus Optionen → Steuerung (dieselbe, dort bleibt sie gleich).
    Bind { id: &'static str },
}

pub struct Module {
    pub name: &'static str,
    pub desc: &'static str,
    pub enable: Enable,
    pub opts: &'static [Opt],
}

pub struct Category {
    pub name: &'static str,
    pub modules: &'static [Module],
}

pub static CATEGORIES: [Category; 4] = [
    Category { name: "GUI", modules: &[] },
    Category {
        name: "Overlays",
        modules: &[
            Module {
                name: "Light Overlay",
                desc: "Red: mobs can always spawn here. Yellow: only at night.",
                enable: Enable::Setting(crate::LIGHT_ON, false),
                opts: &[
                    Opt::Range {
                        key: crate::LIGHT_RANGE,
                        label: "Range",
                        min: 4.0,
                        max: 16.0,
                        step: 1.0,
                        default: 8.0,
                    },
                    Opt::Toggle { key: crate::LIGHT_NIGHT, label: "Show yellow", default: true },
                    Opt::Bind { id: crate::KEY_LIGHT },
                ],
            },
            Module {
                name: "Fullbright",
                desc: "Everything is bright, even in caves and at night.",
                enable: Enable::Setting(crate::FULLBRIGHT, false),
                opts: &[Opt::Bind { id: crate::KEY_FULLBRIGHT }],
            },
            Module {
                name: "No Darkness",
                desc: "No dimming and no fog from the Darkness effect (Warden).",
                enable: Enable::Setting(crate::NO_DARKNESS, false),
                opts: &[Opt::Bind { id: crate::KEY_DARKNESS }],
            },
            Module {
                name: "No Pumpkin",
                desc: "No pumpkin blur when you wear a carved pumpkin.",
                enable: Enable::Setting(crate::NO_PUMPKIN, false),
                opts: &[Opt::Bind { id: crate::KEY_PUMPKIN }],
            },
            Module {
                name: "Weather",
                desc: "Your own weather, only visible to you. The server is not affected.",
                enable: Enable::Setting(crate::WEATHER_ON, false),
                opts: &[
                    Opt::Choice {
                        key: crate::WEATHER_TYPE,
                        label: "Weather",
                        options: &["Clear", "Rain", "Thunder"],
                    },
                    Opt::Bind { id: crate::KEY_WEATHER },
                ],
            },
        ],
    },
    Category {
        name: "Automation",
        modules: &[
            Module {
                name: "Auto Tool",
                desc: "Switches to the fastest fitting tool while mining.",
                enable: Enable::Setting(crate::AUTOTOOL_ON, true),
                opts: &[
                    Opt::Toggle { key: crate::AUTOTOOL_BACK, label: "Switch back", default: true },
                    Opt::Toggle { key: crate::AUTOTOOL_SAVE, label: "Save tools", default: true },
                    Opt::Bind { id: crate::KEY_AUTOTOOL },
                ],
            },
            Module {
                name: "Autoclicker",
                desc: "Clicks for you: left or right, by CPS or at full cooldown.",
                enable: Enable::Clicker,
                opts: &[
                    Opt::Choice {
                        key: crate::CLICKER_BUTTON,
                        label: "Button",
                        options: &["Left", "Right"],
                    },
                    Opt::Range {
                        key: crate::CLICKER_CPS,
                        label: "CPS",
                        min: 1.0,
                        max: 20.0,
                        step: 1.0,
                        default: 10.0,
                    },
                    Opt::Toggle { key: crate::CLICKER_COOLDOWN, label: "Cooldown", default: true },
                    Opt::Toggle { key: crate::OVERLAY_CLICKER, label: "HUD", default: true },
                    Opt::Bind { id: crate::KEY_CLICKER },
                ],
            },
            Module {
                name: "Auto Eat",
                desc: "Eats the best food from your hotbar when you get hungry.",
                enable: Enable::Setting(crate::AUTOEAT_ON, false),
                opts: &[
                    Opt::Range {
                        key: crate::AUTOEAT_HUNGER,
                        label: "Eat at",
                        min: 1.0,
                        max: 19.0,
                        step: 1.0,
                        default: 14.0,
                    },
                    Opt::Toggle { key: crate::AUTOEAT_SAFE, label: "Avoid bad food", default: true },
                    Opt::Bind { id: crate::KEY_AUTOEAT },
                ],
            },
        ],
    },
    Category {
        name: "Conveniences",
        modules: &[
            Module {
                name: "Free Look",
                desc: "Hold the key to look around freely while you keep walking straight.",
                enable: Enable::Setting(crate::FREELOOK_ON, true),
                opts: &[
                    Opt::Choice {
                        key: crate::FREELOOK_MODE,
                        label: "Key mode",
                        options: &["Hold", "Toggle"],
                    },
                    Opt::Range {
                        key: crate::FREELOOK_DIST,
                        label: "Distance",
                        min: 0.0,
                        max: 10.0,
                        step: 1.0,
                        default: 4.0,
                    },
                    Opt::Bind { id: crate::KEY_FREELOOK },
                ],
            },
            Module {
                name: "Easy Bridge",
                desc: "Look down and right-click: the block goes in front of you (like Bedrock).",
                enable: Enable::Setting(crate::BRIDGE_ON, false),
                opts: &[Opt::Bind { id: crate::KEY_BRIDGE }],
            },
        ],
    },
];

/// Was das Menü sich merkt, solange das Spiel läuft.
#[derive(Default)]
pub struct Gui {
    /// Obere linke Ecke jedes Fensters; leer, bis das Menü zum ersten Mal gezeichnet wird.
    pub pos: Option<[[f32; 2]; 4]>,
    /// Breite der Fenster (passt sich beim ersten Öffnen an den Bildschirm an).
    pub width: f32,
    /// Aufgeklappte Module: (Kategorie, Modul).
    pub expanded: Vec<(usize, usize)>,
    /// Das Fenster, das gerade gezogen wird.
    pub dragging: Option<usize>,
    /// Die Taste, die gerade neu belegt wird.
    pub listening: Option<&'static str>,
    /// Der Schieberegler, der gerade gezogen wird: Einstellung, Schiene und Mausposition (x).
    pub sliding: Option<(&'static Opt, f32, f32, f32)>,
}

impl Gui {
    /// Beim Öffnen: alles eingeklappt, nichts in Arbeit.
    pub fn reset(&mut self) {
        self.expanded.clear();
        self.dragging = None;
        self.listening = None;
        self.sliding = None;
    }
}

/// Fensterbreite und -positionen, damit alle vier nebeneinander auf den Bildschirm passen.
pub fn fit(screen_width: f32) -> (f32, [[f32; 2]; 4]) {
    let width = ((screen_width - 2.0 * MARGIN - 3.0 * GAP) / 4.0).clamp(70.0, 110.0);
    let total = 4.0 * width + 3.0 * GAP;
    let left = ((screen_width - total) / 2.0).max(MARGIN);
    let mut pos = [[0.0; 2]; 4];
    for (i, p) in pos.iter_mut().enumerate() {
        *p = [left + i as f32 * (width + GAP), MARGIN + 6.0];
    }
    (width, pos)
}

/// Was an einer Stelle des Menüs liegt.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Target {
    Header(usize),
    Module(usize, usize),
    Opt(usize, usize, usize),
    Empty(usize),
}

/// Eine Zeile im Menü mit ihrem Rechteck `[x, y, w, h]`.
pub struct Row {
    pub rect: [f32; 4],
    pub target: Target,
}

/// Alle Zeilen aller Fenster, von oben nach unten.
pub fn layout(gui: &Gui) -> Vec<Row> {
    let mut rows = Vec::new();
    let Some(pos) = gui.pos else {
        return rows;
    };
    let w = gui.width;
    for (c, cat) in CATEGORIES.iter().enumerate() {
        let [x, mut y] = pos[c];
        rows.push(Row { rect: [x, y, w, HEADER_H], target: Target::Header(c) });
        y += HEADER_H;
        if cat.modules.is_empty() {
            rows.push(Row { rect: [x, y, w, ROW_H], target: Target::Empty(c) });
            continue;
        }
        for (m, module) in cat.modules.iter().enumerate() {
            rows.push(Row { rect: [x, y, w, ROW_H], target: Target::Module(c, m) });
            y += ROW_H;
            if gui.expanded.contains(&(c, m)) {
                for (o, opt) in module.opts.iter().enumerate() {
                    let h = opt_height(opt);
                    rows.push(Row { rect: [x, y, w, h], target: Target::Opt(c, m, o) });
                    y += h;
                }
            }
        }
    }
    rows
}

/// Zahlen bekommen eine Zeile mehr für den Schieberegler.
fn opt_height(opt: &Opt) -> f32 {
    if matches!(opt, Opt::Range { .. }) { OPT_H + SLIDER_H } else { OPT_H }
}

/// Die Schiene eines Schiebereglers in einer Zeile `[x, y, w, h]`: von, bis, Höhe (y).
fn slider_track(rect: [f32; 4]) -> (f32, f32, f32) {
    let [x, y, w, h] = rect;
    (x + 8.0, x + w - 6.0, y + h - 5.0)
}

/// Der Wert an der Mausposition `mx` auf der Schiene, auf `step` gerundet.
pub fn slider_value(mx: f32, from: f32, to: f32, min: f64, max: f64, step: f64) -> f64 {
    let t = f64::from(((mx - from) / (to - from).max(1.0)).clamp(0.0, 1.0));
    let raw = min + t * (max - min);
    let stepped = if step > 0.0 { min + ((raw - min) / step).round() * step } else { raw };
    stepped.clamp(min, max)
}

pub fn hit(rows: &[Row], at: [f32; 2]) -> Option<&Row> {
    rows.iter().rev().find(|r| {
        let [x, y, w, h] = r.rect;
        at[0] >= x && at[0] < x + w && at[1] >= y && at[1] < y + h
    })
}

// ------------------------------------------------------------ Werte

fn module_on(state: &State, module: &Module) -> bool {
    match module.enable {
        Enable::Setting(key, default) => state.cfg.get_bool(key, default),
        Enable::Clicker => state.clicker_on,
    }
}

/// Eine Taste aus Steuerung als kurzer Name: `key.keyboard.right.shift` → `RIGHT SHIFT`.
pub fn key_name(key: &str) -> String {
    if key == UNBOUND || key.is_empty() {
        return "-".to_owned();
    }
    if let Some(button) = key.strip_prefix("key.mouse.") {
        return format!("MOUSE {}", button.to_uppercase());
    }
    key.strip_prefix("key.keyboard.")
        .unwrap_or(key)
        .replace('.', " ")
        .to_uppercase()
}

fn opt_text(host: &dyn ClientHost, state: &State, opt: &Opt) -> (String, String, u32) {
    match opt {
        Opt::Toggle { key, label, default } => {
            let on = state.cfg.get_bool(key, *default);
            (label.to_string(), if on { "On" } else { "Off" }.into(), if on { ON } else { TEXT_DIM })
        }
        Opt::Choice { key, label, options } => {
            let i = state.cfg.get_usize(key, 0);
            (label.to_string(), options.get(i).copied().unwrap_or("?").into(), ON)
        }
        Opt::Range { key, label, default, .. } => {
            let v = state.cfg.get_f64(key, *default);
            (label.to_string(), format!("{v:.0}"), ON)
        }
        Opt::Bind { id } => {
            if state.gui.listening == Some(*id) {
                return ("Key".into(), "...".into(), LISTEN);
            }
            let key = host.mod_key(MOD_ID, id).map(|(k, _)| k).unwrap_or_default();
            ("Key".into(), key_name(&key), TEXT)
        }
    }
}

/// Ändert eine Einstellung: `up` = vorwärts/mehr, sonst rückwärts/weniger.
fn change_opt(host: &dyn ClientHost, state: &mut State, opt: &'static Opt, up: bool) {
    match opt {
        Opt::Toggle { key, default, .. } => {
            let on = state.cfg.get_bool(key, *default);
            state.cfg.set(host, key, (!on).to_string());
        }
        Opt::Choice { key, options, .. } => {
            let n = options.len().max(1);
            let i = state.cfg.get_usize(key, 0) % n;
            let next = if up { (i + 1) % n } else { (i + n - 1) % n };
            state.cfg.set(host, key, next.to_string());
        }
        Opt::Range { key, min, max, step, default, .. } => {
            let v = state.cfg.get_f64(key, *default);
            state.cfg.set(host, key, step_range(v, *min, *max, *step, up).to_string());
        }
        Opt::Bind { id } => {
            state.gui.listening = Some(*id);
        }
    }
}

pub fn step_range(value: f64, min: f64, max: f64, step: f64, up: bool) -> f64 {
    let next = if up { value + step } else { value - step };
    next.clamp(min, max)
}

// ------------------------------------------------------------ Zeichnen

/// Text, der in `max` Pixel passt (sonst kleiner geschrieben).
fn fitted_text(draw: &mut dyn DrawContext, text: &str, at: [f32; 2], max: f32, argb: u32, shadow: bool) {
    let w = draw.text_width(text);
    if w <= max || w <= 0.0 {
        draw.text(text, at, argb, shadow);
    } else {
        draw.text_scaled(text, [at[0], at[1] + 1.0], (max / w).max(0.5), argb, shadow);
    }
}

pub fn draw(host: &dyn ClientHost, state: &mut State, draw: &mut dyn DrawContext, mouse: [f32; 2]) {
    let [sw, sh] = draw.size();
    if state.gui.pos.is_none() {
        let (width, pos) = fit(sw);
        state.gui.width = width;
        state.gui.pos = Some(pos);
    }
    draw.fill([0.0, 0.0, sw, sh], SCREEN_DIM);

    let rows = layout(&state.gui);
    let hovered = hit(&rows, mouse).map(|r| r.target);
    let mut hover_desc = None;

    for row in &rows {
        let [x, y, w, h] = row.rect;
        let hover = hovered == Some(row.target);
        match row.target {
            Target::Header(c) => {
                draw.fill([x, y, w, h], HEADER);
                let name = CATEGORIES[c].name;
                let tw = draw.text_width(name).min(w - 4.0);
                fitted_text(draw, name, [x + (w - tw) / 2.0, y + 4.0], w - 4.0, 0xFFFF_FFFF, true);
            }
            Target::Empty(_) => {
                draw.fill([x, y, w, h], BG);
                draw.text("empty", [x + 5.0, y + 3.0], TEXT_DIM, false);
            }
            Target::Module(c, m) => {
                let module = &CATEGORIES[c].modules[m];
                let on = module_on(state, module);
                draw.fill([x, y, w, h], BG);
                if hover {
                    draw.fill([x, y, w, h], ROW_HOVER);
                    hover_desc = Some(module.desc);
                }
                if on {
                    draw.fill([x, y, 2.0, h], ON);
                }
                fitted_text(draw, module.name, [x + 5.0, y + 3.0], w - 16.0, if on { ON } else { TEXT }, false);
                if !module.opts.is_empty() {
                    let open = state.gui.expanded.contains(&(c, m));
                    draw.text(if open { "-" } else { "+" }, [x + w - 8.0, y + 3.0], TEXT_DIM, false);
                }
            }
            Target::Opt(c, m, o) => {
                let opt = &CATEGORIES[c].modules[m].opts[o];
                let (label, value, color) = opt_text(host, state, opt);
                draw.fill([x, y, w, h], OPT_BG);
                if hover {
                    draw.fill([x, y, w, h], ROW_HOVER);
                }
                let vw = draw.text_width(&value).min(w / 2.0);
                fitted_text(draw, &label, [x + 8.0, y + 2.0], w - vw - 14.0, TEXT_DIM, false);
                fitted_text(draw, &value, [x + w - vw - 4.0, y + 2.0], w / 2.0, color, false);
                if let Opt::Range { key, min, max, default, .. } = opt {
                    let v = state.cfg.get_f64(key, *default);
                    let (from, to, ty) = slider_track(row.rect);
                    let t = if max > min { ((v - min) / (max - min)).clamp(0.0, 1.0) as f32 } else { 0.0 };
                    let knob = from + t * (to - from);
                    draw.fill([from, ty, to - from, 2.0], TRACK);
                    draw.fill([from, ty, knob - from, 2.0], ON);
                    draw.fill([knob - 1.5, ty - 2.0, 3.0, 6.0], 0xFFFF_FFFF);
                }
            }
        }
    }

    let help = if state.gui.listening.is_some() {
        "Press a new key  ·  Esc: cancel  ·  Delete: no key"
    } else {
        hover_desc.unwrap_or("Left: on/off  ·  Right: settings  ·  Drag title to move  ·  Esc: close")
    };
    draw.fill([0.0, sh - 14.0, sw, 14.0], 0xC000_0000);
    let tw = draw.text_width(help).min(sw - 4.0);
    fitted_text(draw, help, [(sw - tw) / 2.0, sh - 11.0], sw - 4.0, TEXT, false);
}

// ------------------------------------------------------------ Eingaben

/// Ein Mausklick im Menü. `button`: 0 links, 1 rechts.
pub fn click(host: &dyn ClientHost, state: &mut State, at: [f32; 2], button: i32) {
    // Beim Neubelegen ist ein Mausklick die neue Taste (Maustasten 3–8, z. B. Seitentasten).
    if let Some(id) = state.gui.listening.take() {
        if button >= 2 {
            state.pending_bind = Some((id, format!("key.mouse.{}", button + 1)));
            return;
        }
    }
    let rows = layout(&state.gui);
    let Some(row) = hit(&rows, at) else {
        return;
    };
    let rect = row.rect;
    match row.target {
        Target::Header(c) => {
            if button == 0 {
                state.gui.dragging = Some(c);
            }
        }
        Target::Empty(_) => {}
        Target::Module(c, m) => {
            let module = &CATEGORIES[c].modules[m];
            if button == 1 {
                if let Some(i) = state.gui.expanded.iter().position(|e| *e == (c, m)) {
                    state.gui.expanded.remove(i);
                } else if !module.opts.is_empty() {
                    state.gui.expanded.push((c, m));
                }
            } else if button == 0 {
                match module.enable {
                    Enable::Setting(key, default) => {
                        let on = state.cfg.get_bool(key, default);
                        state.cfg.set(host, key, (!on).to_string());
                    }
                    Enable::Clicker => {
                        state.clicker_on = !state.clicker_on;
                        state.pending_clicks = 0;
                        state.ticks_since_click = u32::MAX / 2;
                    }
                }
            }
        }
        Target::Opt(c, m, o) => {
            let opt = &CATEGORIES[c].modules[m].opts[o];
            if let Opt::Range { key, min, max, step, .. } = opt {
                if button == 0 {
                    let (from, to, _) = slider_track(rect);
                    let v = slider_value(at[0], from, to, *min, *max, *step);
                    state.cfg.set(host, key, v.to_string());
                    state.gui.sliding = Some((opt, from, to, at[0]));
                }
                return;
            }
            change_opt(host, state, opt, button == 0);
        }
    }
}

/// Mausrad über einer Zahlen- oder Auswahl-Einstellung.
pub fn scroll(host: &dyn ClientHost, state: &mut State, at: [f32; 2], amount: f64) {
    let rows = layout(&state.gui);
    let Some(Target::Opt(c, m, o)) = hit(&rows, at).map(|r| r.target) else {
        return;
    };
    let opt = &CATEGORIES[c].modules[m].opts[o];
    if matches!(opt, Opt::Range { .. } | Opt::Choice { .. }) && amount != 0.0 {
        change_opt(host, state, opt, amount > 0.0);
    }
}

/// Eine Taste, während das Menü offen ist. `true`, wenn das Menü sie genommen hat.
pub fn key_pressed(state: &mut State, key: &str) -> bool {
    let Some(id) = state.gui.listening else {
        return false;
    };
    state.gui.listening = None;
    match key {
        "key.keyboard.escape" => {}
        "key.keyboard.delete" | "key.keyboard.backspace" => {
            state.pending_bind = Some((id, UNBOUND.to_owned()));
        }
        _ => state.pending_bind = Some((id, key.to_owned())),
    }
    true
}

/// Fenster an der Titelleiste ziehen, oder einen Schieberegler.
pub fn drag(host: &dyn ClientHost, state: &mut State, delta: [f32; 2]) {
    if let Some((opt, from, to, mx)) = state.gui.sliding {
        let mx = mx + delta[0];
        state.gui.sliding = Some((opt, from, to, mx));
        if let Opt::Range { key, min, max, step, .. } = opt {
            let v = slider_value(mx, from, to, *min, *max, *step);
            if (v - state.cfg.get_f64(key, *min)).abs() > f64::EPSILON {
                state.cfg.set(host, key, v.to_string());
            }
        }
        return;
    }
    let (Some(c), Some(pos)) = (state.gui.dragging, state.gui.pos.as_mut()) else {
        return;
    };
    pos[c][0] += delta[0];
    pos[c][1] = (pos[c][1] + delta[1]).max(0.0);
}

pub fn release(state: &mut State) {
    state.gui.dragging = None;
    state.gui.sliding = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opened() -> Gui {
        let (width, pos) = fit(427.0);
        Gui { pos: Some(pos), width, ..Gui::default() }
    }

    #[test]
    fn all_four_panels_fit_on_a_small_gui_screen() {
        for screen in [320.0f32, 427.0, 480.0, 640.0, 960.0] {
            let (width, pos) = fit(screen);
            let right = pos[3][0] + width;
            assert!(pos[0][0] >= 0.0 && right <= screen.max(4.0 * 70.0 + 3.0 * GAP + 2.0 * MARGIN), "{screen}");
        }
    }

    #[test]
    fn expanding_a_module_adds_its_options_below_it() {
        let mut gui = opened();
        let closed = layout(&gui).len();
        gui.expanded.push((2, 1)); // Autoclicker: 5 options
        let rows = layout(&gui);
        assert_eq!(rows.len(), closed + 5);
        let i = rows.iter().position(|r| r.target == Target::Module(2, 1)).unwrap();
        assert_eq!(rows[i + 1].target, Target::Opt(2, 1, 0));
    }

    #[test]
    fn clicks_find_the_row_under_the_mouse() {
        let gui = opened();
        let rows = layout(&gui);
        let [x, y] = gui.pos.unwrap()[2];
        assert_eq!(hit(&rows, [x + 5.0, y + HEADER_H + 2.0]).map(|r| r.target), Some(Target::Module(2, 0)));
        assert_eq!(hit(&rows, [x + 5.0, y + 2.0]).map(|r| r.target), Some(Target::Header(2)));
    }

    #[test]
    fn key_names_are_short() {
        assert_eq!(key_name("key.keyboard.right.shift"), "RIGHT SHIFT");
        assert_eq!(key_name("key.keyboard.k"), "K");
        assert_eq!(key_name("key.mouse.4"), "MOUSE 4");
        assert_eq!(key_name(UNBOUND), "-");
    }

    #[test]
    fn slider_maps_mouse_to_steps() {
        assert_eq!(slider_value(0.0, 10.0, 110.0, 1.0, 20.0, 1.0), 1.0);
        assert_eq!(slider_value(200.0, 10.0, 110.0, 1.0, 20.0, 1.0), 20.0);
        assert_eq!(slider_value(60.0, 10.0, 110.0, 0.0, 10.0, 1.0), 5.0);
    }

    #[test]
    fn reopening_collapses_everything() {
        let mut gui = opened();
        gui.expanded.push((2, 1));
        gui.reset();
        assert!(gui.expanded.is_empty());
    }

    #[test]
    fn ranges_stay_inside_their_bounds() {
        assert_eq!(step_range(20.0, 1.0, 20.0, 1.0, true), 20.0);
        assert_eq!(step_range(1.0, 1.0, 20.0, 1.0, false), 1.0);
    }
}
