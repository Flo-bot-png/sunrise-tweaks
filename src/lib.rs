//! Sunrise Tweaks: viele kleine Client-Verbesserungen in einer Mod.
//!
//! - **Klick-Menü** (Standard: rechte Shift-Taste, siehe `gui.rs`): alle Module nach Kategorie,
//!   mit ihren Einstellungen und Tasten. Die Taste für das Menü selbst stellt man unter
//!   Optionen → Steuerung oder auf der Config-Seite im Mod-Menü ein.
//! - **Auto-Tool:** beim Abbauen das schnellste passende Werkzeug aus der Hotbar.
//! - **Autoclicker:** links oder rechts, feste CPS oder immer bei vollem Angriffs-Cooldown.
//!
//! Alles läuft über die normalen Tasten des Spiels (`click_mapping`, `select_hotbar_slot`):
//! der Server sieht dieselben Pakete wie bei einem Spieler, der selbst drückt.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

mod config;
mod gui;

use mod_api::*;

const MOD_ID: &str = "sunrise_tweaks";

// Einstellungen (gespeichert von `config.rs`, geändert im Klick-Menü)
const AUTOTOOL_ON: &str = "autotool.enabled";
const AUTOTOOL_BACK: &str = "autotool.switch_back";
const AUTOTOOL_SAVE: &str = "autotool.save_tools";
const CLICKER_BUTTON: &str = "autoclicker.button";
const CLICKER_CPS: &str = "autoclicker.cps";
const CLICKER_COOLDOWN: &str = "autoclicker.cooldown";
const OVERLAY_CLICKER: &str = "overlay.clicker_hint";
const LIGHT_ON: &str = "overlay.light.enabled";
const LIGHT_RANGE: &str = "overlay.light.range";
const LIGHT_NIGHT: &str = "overlay.light.night";
const NO_DARKNESS: &str = "overlay.no_darkness";
const FULLBRIGHT: &str = "overlay.fullbright";
const NO_PUMPKIN: &str = "overlay.no_pumpkin";
const WEATHER_ON: &str = "overlay.weather.enabled";
const WEATHER_TYPE: &str = "overlay.weather.type";
const AUTOEAT_ON: &str = "autoeat.enabled";
const AUTOEAT_HUNGER: &str = "autoeat.hunger";
const AUTOEAT_SAFE: &str = "autoeat.avoid_bad";
const FREELOOK_ON: &str = "freelook.enabled";
const FREELOOK_MODE: &str = "freelook.mode";
const FREELOOK_DIST: &str = "freelook.distance";
const BRIDGE_ON: &str = "easybridge.enabled";

// Die einzige Einstellung auf der Config-Seite im Mod-Menü: die Menü-Taste.
const MENU_KEY: &str = "menu_key";
/// Die Tasten, die man auf der Config-Seite wählen kann (Option 0 = wie in Steuerung).
const MENU_KEYS: [&str; 5] = [
    "key.keyboard.right.shift",
    "key.keyboard.right.control",
    "key.keyboard.insert",
    "key.keyboard.home",
    "key.keyboard.end",
];

// Tasten (Optionen → Steuerung)
const KEY_CLICKER: &str = "toggle_autoclicker";
const KEY_AUTOTOOL: &str = "toggle_autotool";
const KEY_LIGHT: &str = "toggle_light_overlay";
const KEY_DARKNESS: &str = "toggle_no_darkness";
const KEY_FULLBRIGHT: &str = "toggle_fullbright";
const KEY_PUMPKIN: &str = "toggle_no_pumpkin";
const KEY_WEATHER: &str = "toggle_weather";
const KEY_AUTOEAT: &str = "toggle_autoeat";
const KEY_FREELOOK: &str = "freelook";
const KEY_BRIDGE: &str = "toggle_easybridge";

// ---------------------------------------------------------------- Für die Eingriffe ins Spiel
// Die Helfer in `src/mixins/` laufen im Code des Spiels und fragen hier, ob ihr Modul an ist.
// Die Werte werden jeden Tick aus der Konfiguration übernommen.

static NO_DARKNESS_ON: AtomicBool = AtomicBool::new(false);
static FULLBRIGHT_ON: AtomicBool = AtomicBool::new(false);
static NO_PUMPKIN_ON: AtomicBool = AtomicBool::new(false);
/// Weather Changer: 0 = aus, 1 = klar, 2 = Regen, 3 = Gewitter.
static WEATHER_MODE: AtomicU8 = AtomicU8::new(0);
static BRIDGE_ACTIVE: AtomicBool = AtomicBool::new(false);

/// No Darkness an? (für `src/mixins/no_darkness_*.rs`)
pub fn no_darkness() -> bool {
    NO_DARKNESS_ON.load(Ordering::Relaxed)
}
/// Fullbright an? (für `src/mixins/fullbright.rs`)
pub fn fullbright() -> bool {
    FULLBRIGHT_ON.load(Ordering::Relaxed)
}
/// No Pumpkin Overlay an? (für `src/mixins/no_pumpkin.rs`)
pub fn no_pumpkin() -> bool {
    NO_PUMPKIN_ON.load(Ordering::Relaxed)
}
/// Das Wetter, das man selbst sieht (für `src/mixins/weather.rs`): Regen- und Gewitterstärke.
pub fn weather_view(world: (f32, f32)) -> (f32, f32) {
    match WEATHER_MODE.load(Ordering::Relaxed) {
        1 => (0.0, 0.0),
        2 => (1.0, 0.0),
        3 => (1.0, 1.0),
        _ => world,
    }
}
/// Keine Taste.
const UNBOUND: &str = "key.keyboard.unknown";

/// Unter so viel Rest-Haltbarkeit nimmt Auto-Tool ein Werkzeug nicht mehr (wenn „schonen“ an ist).
const MIN_DURABILITY: u32 = 5;

#[derive(Default)]
struct Mod {
    state: Mutex<State>,
}

#[derive(Default)]
struct State {
    cfg: config::Config,
    /// Slot vor dem automatischen Wechsel, um danach zurückzuwechseln.
    previous_slot: Option<u8>,
    /// Autoclicker an?
    clicker_on: bool,
    /// Angesammelte Klick-Bruchteile in Zwanzigsteln (ganze Zahlen, keine Rundungsfehler).
    pending_clicks: u32,
    /// Ticks seit dem letzten Klick des Autoclickers (für „voller Cooldown“).
    ticks_since_click: u32,
    /// Das Klick-Menü.
    gui: gui::Gui,
    /// Eine Taste, die im Klick-Menü neu belegt wurde: wird im nächsten Tick gespeichert.
    pending_bind: Option<(&'static str, String)>,
    /// Light Overlay: die gefundenen Spawn-Stellen (Block, nur nachts?) und wann zuletzt gesucht.
    light_spots: Vec<([i32; 3], bool)>,
    light_age: u32,
    /// Auto Eat: gerade am Essen (Slot vorher, Ticks seit dem Start, Hunger beim Start).
    eating: Option<Eating>,
    /// Free Look: gerade aktiv?
    freelook: Option<FreeLook>,
    /// Free Look „Umschalten“: per Taste an?
    freelook_toggled: bool,
    /// Augenposition des Spielers im letzten und vorletzten Tick (für die Kamera zwischen Ticks).
    eye: [[f64; 3]; 2],
}

struct Eating {
    previous_slot: Option<u8>,
    ticks: u32,
    start_food: i32,
}

/// Free Look: wohin der Spieler schaut (bleibt fest) und wohin die Kamera schaut.
struct FreeLook {
    player: [f32; 2],
    camera: [f32; 2],
    /// Die Kamera-Ansicht vor Free Look, um sie danach wiederherzustellen.
    previous_view: Option<CameraType>,
}

#[mod_api::sunrise_mod]
impl Hooks for Mod {
    fn register(&self, r: &mut dyn Registrar, _: &RegistrationContext) {
        let keys = [
            (gui::KEY_GUI, "key.keyboard.right.shift", "key.sunrise_tweaks.open_gui", "gui"),
            (KEY_CLICKER, "key.keyboard.k", "key.sunrise_tweaks.toggle_autoclicker", "automation"),
            (KEY_AUTOTOOL, UNBOUND, "key.sunrise_tweaks.toggle_autotool", "automation"),
            (KEY_LIGHT, UNBOUND, "key.sunrise_tweaks.toggle_light_overlay", "overlays"),
            (KEY_DARKNESS, UNBOUND, "key.sunrise_tweaks.toggle_no_darkness", "overlays"),
            (KEY_FULLBRIGHT, UNBOUND, "key.sunrise_tweaks.toggle_fullbright", "overlays"),
            (KEY_PUMPKIN, UNBOUND, "key.sunrise_tweaks.toggle_no_pumpkin", "overlays"),
            (KEY_WEATHER, UNBOUND, "key.sunrise_tweaks.toggle_weather", "overlays"),
            (KEY_AUTOEAT, UNBOUND, "key.sunrise_tweaks.toggle_autoeat", "automation"),
            (KEY_FREELOOK, "key.keyboard.left.alt", "key.sunrise_tweaks.freelook", "conveniences"),
            (KEY_BRIDGE, UNBOUND, "key.sunrise_tweaks.toggle_easybridge", "conveniences"),
        ];
        for (id, key, label, category) in keys {
            r.register_key_mapping(
                KeyRegistration::new(id, key)
                    .label(label)
                    .category(&format!("sunrise_tweaks.category.{category}"))
                    .context(KeyConflictContext::InGame),
            );
        }

        // Config-Seite im Mod-Menü: nur noch die Menü-Taste.
        let mut options = vec!["sunrise_tweaks.menu_key.controls"];
        options.extend(MENU_KEYS.iter().map(|k| *k));
        let _ = r.register_setting(
            SettingSpec::choice(MENU_KEY, "sunrise_tweaks.setting.menu_key", options, 0)
                .tooltip("sunrise_tweaks.setting.menu_key.tooltip"),
        );
    }

    /// Auf der Config-Seite eine Menü-Taste gewählt: in Steuerung übernehmen.
    fn setting_changed(&self, host: &mut dyn ClientHost, event: &mut SettingChangedEvent<'_>) {
        if event.key != MENU_KEY {
            return;
        }
        let choice = host.setting_choice(MENU_KEY).unwrap_or(0) as usize;
        if let Some(key) = choice.checked_sub(1).and_then(|i| MENU_KEYS.get(i)) {
            let _ = host.set_mod_key(MOD_ID, gui::KEY_GUI, key, ClientModifiers::default());
        }
    }

    /// Vor den Eingaben des Ticks: Auto-Tool wählt den Slot, bevor der Schlag rausgeht.
    fn client_tick_pre(&self, host: &mut dyn ClientHost, _: &mut ClientTickPreEvent) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.cfg.ensure_loaded(&*host);
        auto_tool(host, &mut state);
        auto_eat(host, &mut state);
        free_look_tick(host, &mut state);
    }

    /// Nach dem Tick: Menü öffnen, Tasten speichern, Module per Taste umschalten, klicken.
    fn client_tick_post(&self, host: &mut dyn ClientHost, _: &mut ClientTickPostEvent) {
        let mut open = false;
        while host.consume_click(MOD_ID, gui::KEY_GUI) {
            open = true;
        }

        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.cfg.ensure_loaded(&*host);

        if open && host.playing() && host.mod_screen().is_none() {
            // Jedes Mal frisch: alle Module eingeklappt.
            state.gui.reset();
            host.mod_screen_open(MOD_ID);
        }

        if let Some((id, key)) = state.pending_bind.take() {
            let _ = host.set_mod_key(MOD_ID, id, &key, ClientModifiers::default());
        }

        while host.consume_click(MOD_ID, KEY_CLICKER) {
            toggle_clicker(host, &mut state);
        }
        while host.consume_click(MOD_ID, KEY_AUTOTOOL) {
            let on = !state.cfg.get_bool(AUTOTOOL_ON, true);
            state.cfg.set(&*host, AUTOTOOL_ON, on.to_string());
            let _ = host.chat_line(Text::literal(if on { "Auto Tool on" } else { "Auto Tool off" }));
        }
        for (id, key, default, name) in [
            (KEY_LIGHT, LIGHT_ON, false, "Light Overlay"),
            (KEY_DARKNESS, NO_DARKNESS, false, "No Darkness"),
            (KEY_FULLBRIGHT, FULLBRIGHT, false, "Fullbright"),
            (KEY_PUMPKIN, NO_PUMPKIN, false, "No Pumpkin Overlay"),
            (KEY_WEATHER, WEATHER_ON, false, "Weather Changer"),
            (KEY_AUTOEAT, AUTOEAT_ON, false, "Auto Eat"),
            (KEY_BRIDGE, BRIDGE_ON, false, "Easy Bridge"),
        ] {
            while host.consume_click(MOD_ID, id) {
                let on = !state.cfg.get_bool(key, default);
                state.cfg.set(&*host, key, on.to_string());
                let line = format!("{name} {}", if on { "on" } else { "off" });
                let _ = host.chat_line(Text::literal(line));
            }
        }

        NO_DARKNESS_ON.store(state.cfg.get_bool(NO_DARKNESS, false), Ordering::Relaxed);
        FULLBRIGHT_ON.store(state.cfg.get_bool(FULLBRIGHT, false), Ordering::Relaxed);
        NO_PUMPKIN_ON.store(state.cfg.get_bool(NO_PUMPKIN, false), Ordering::Relaxed);
        let weather = if state.cfg.get_bool(WEATHER_ON, false) {
            state.cfg.get_usize(WEATHER_TYPE, 0).min(2) as u8 + 1
        } else {
            0
        };
        WEATHER_MODE.store(weather, Ordering::Relaxed);
        BRIDGE_ACTIVE.store(state.cfg.get_bool(BRIDGE_ON, false), Ordering::Relaxed);

        // Free Look „Umschalten“: Taste schaltet an/aus; „Halten“: Klicks verwerfen.
        while host.consume_click(MOD_ID, KEY_FREELOOK) {
            if state.cfg.get_usize(FREELOOK_MODE, 0) == 1 {
                state.freelook_toggled = !state.freelook_toggled;
            }
        }
        // Augenposition für die Free-Look-Kamera merken (nach der Bewegung dieses Ticks).
        if let Some(eye) = client_eye(&*host) {
            state.eye = [state.eye[1], eye];
        }

        auto_clicker(host, &mut state);
        light_scan(&*host, &mut state);
    }

    /// Free Look: die Kamera dreht sich mit der Maus, der Spieler schaut weiter geradeaus.
    fn camera_pose(&self, host: &dyn ClientHost, event: &mut CameraPoseEvent<'_>) {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let Some(look) = &state.freelook else {
            return;
        };
        let live = host
            .local_player()
            .and_then(|p| p.entity().rotation(host))
            .unwrap_or(look.player);
        // Was die Maus seit dem letzten Tick gedreht hat, kommt noch dazu.
        let yaw = look.camera[0] + (live[0] - look.player[0]);
        let pitch = (look.camera[1] + (live[1] - look.player[1])).clamp(-90.0, 90.0);
        event.yaw = yaw;
        event.pitch = pitch;

        let distance = state.cfg.get_f64(FREELOOK_DIST, 4.0).clamp(0.0, 10.0);
        if distance > 0.0 {
            let t = f64::from(event.partial.clamp(0.0, 1.0));
            let [a, b] = state.eye;
            let pivot = [0, 1, 2].map(|i| a[i] + (b[i] - a[i]) * t);
            let dir = look_dir(yaw, pitch);
            *event.position = [0, 1, 2].map(|i| pivot[i] - dir[i] * distance);
        }
    }

    /// Easy Bridge: beim Bauen nach unten-vorne zielt das Fadenkreuz auf die Seite des Blocks
    /// unter dir, so dass der neue Block vor dir landet (wie in Bedrock).
    fn client_pick(&self, host: &mut dyn ClientHost, event: &mut ClientPickEvent<'_>) {
        if !BRIDGE_ACTIVE.load(Ordering::Relaxed) {
            return;
        }
        let client: &dyn ClientHost = &*host;
        if client.mapping_physical_down("key.use") != Ok(Some(true)) {
            return;
        }
        if let Some(target) = bridge_target(client, event.result.as_ref()) {
            *event.result = Some(target);
        }
    }

    /// Light Overlay: die gefundenen Stellen als flache Vierecke auf den Boden zeichnen.
    fn render_level_stage(&self, _: &dyn ClientHost, event: &mut RenderLevelStageEvent<'_>) {
        if event.stage != RenderLevelStage::AfterTranslucentBlocks {
            return;
        }
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if !state.cfg.get_bool(LIGHT_ON, false) || state.light_spots.is_empty() {
            return;
        }
        let quads: Vec<WorldQuad> = state
            .light_spots
            .iter()
            .map(|&(pos, night_only)| light_quad(pos, night_only))
            .collect();
        let _ = event.draw.quads(&quads);
    }

    /// No Darkness: solange der Dunkelheits-Effekt wirkt, normaler Nebel statt der dunklen Wand.
    fn render_fog(&self, host: &dyn ClientHost, event: &mut RenderFogEvent<'_>) {
        if event.fog_type != client::fog::FogType::Atmospheric {
            return;
        }
        let on = self
            .state
            .lock()
            .map(|s| s.cfg.get_bool(NO_DARKNESS, false))
            .unwrap_or(false);
        if !on || !has_darkness(host) {
            return;
        }
        let fog = &mut *event.fog;
        fog.environmental_start = fog.render_distance_start;
        fog.environmental_end = fog.render_distance_end;
    }

    /// Kleiner Hinweis oben links, solange der Autoclicker läuft.
    fn gui_render_post(&self, _: &dyn ClientHost, event: &mut GuiRenderPostEvent<'_>) {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if event.gui_hidden || !state.clicker_on || !state.cfg.get_bool(OVERLAY_CLICKER, true) {
            return;
        }
        let label = match state.cfg.get_usize(CLICKER_BUTTON, 0) {
            1 => "Autoclicker: Right",
            _ => "Autoclicker: Left",
        };
        let width = event.batch.text_width(label);
        event.batch.fill([2.0, 2.0, width + 6.0, 12.0], 0x8000_0000);
        event.batch.text(label, [5.0, 4.0], 0xFFB0_8CFF, true);
    }

    // ---------- Klick-Menü: zeichnen und Eingaben (nur wenn es unser Menü ist)

    fn screen_render_post(&self, host: &dyn ClientHost, event: &mut ScreenRenderPostEvent<'_>) {
        if host.mod_screen() != Some(MOD_ID) {
            return;
        }
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        gui::draw(host, &mut state, &mut *event.draw, event.screen.mouse);
    }

    fn screen_mouse_button_pressed_pre(
        &self,
        host: &dyn ClientHost,
        event: &mut ScreenMouseButtonPressedPreEvent<'_>,
    ) -> Flow {
        if host.mod_screen() != Some(MOD_ID) {
            return Flow::Continue;
        }
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        gui::click(host, &mut state, event.screen.mouse, event.button);
        Flow::Cancel
    }

    fn screen_mouse_button_released_pre(
        &self,
        host: &dyn ClientHost,
        _: &mut ScreenMouseButtonReleasedPreEvent<'_>,
    ) -> Flow {
        if host.mod_screen() != Some(MOD_ID) {
            return Flow::Continue;
        }
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        gui::release(&mut state);
        Flow::Cancel
    }

    fn screen_mouse_dragged_pre(
        &self,
        host: &dyn ClientHost,
        event: &mut ScreenMouseDraggedPreEvent<'_>,
    ) -> Flow {
        if host.mod_screen() != Some(MOD_ID) {
            return Flow::Continue;
        }
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        gui::drag(host, &mut state, event.delta);
        Flow::Cancel
    }

    fn screen_mouse_scrolled_pre(
        &self,
        host: &dyn ClientHost,
        event: &mut ScreenMouseScrolledPreEvent<'_>,
    ) -> Flow {
        if host.mod_screen() != Some(MOD_ID) {
            return Flow::Continue;
        }
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        gui::scroll(host, &mut state, event.screen.mouse, event.scroll[1]);
        Flow::Cancel
    }

    /// Taste neu belegen: die nächste gedrückte Taste wird die neue Taste.
    fn screen_key_pressed_pre(
        &self,
        host: &dyn ClientHost,
        event: &mut ScreenKeyPressedPreEvent<'_>,
    ) -> Flow {
        if host.mod_screen() != Some(MOD_ID) {
            return Flow::Continue;
        }
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if gui::key_pressed(&mut state, event.key) {
            Flow::Cancel
        } else {
            Flow::Continue
        }
    }
}

// ---------------------------------------------------------------- Overlays

/// Höchstens so viele Stellen zeichnet das Light Overlay (das Spiel erlaubt 4096 Vierecke).
const MAX_LIGHT_SPOTS: usize = 3000;
/// Alle so viele Ticks wird neu gesucht (halbe Sekunde).
const LIGHT_SCAN_TICKS: u32 = 10;

/// Sucht Stellen um den Spieler, an denen Mobs spawnen können: Luft (oder Gras o. Ä.)
/// auf einem vollen Block, Blocklicht 0. Ohne Himmelslicht ist es rot (immer), sonst gelb (nachts).
fn light_scan(client: &dyn ClientHost, state: &mut State) {
    if !state.cfg.get_bool(LIGHT_ON, false) {
        state.light_spots.clear();
        return;
    }
    state.light_age += 1;
    if state.light_age < LIGHT_SCAN_TICKS {
        return;
    }
    state.light_age = 0;

    let host: &dyn Host = client;
    let Some(player) = client.local_player() else {
        return;
    };
    let (Some(level), Some(at)) = (player.level(host), player.entity().block_position(host)) else {
        return;
    };
    let range = state.cfg.get_f64(LIGHT_RANGE, 8.0).clamp(4.0, 16.0) as i32;
    let show_night = state.cfg.get_bool(LIGHT_NIGHT, true);
    let facts = host.block_states();
    let [px, py, pz] = at;

    let mut spots = Vec::new();
    'scan: for dy in -6..=4 {
        for dx in -range..=range {
            for dz in -range..=range {
                let pos = [px + dx, py + dy, pz + dz];
                let below = [pos[0], pos[1] - 1, pos[2]];
                let (Some(here), Some(ground)) =
                    (level.block_state(host, pos), level.block_state(host, below))
                else {
                    continue;
                };
                let open = facts.is_solid_render(here) == Some(false)
                    && facts.replaceable(here) == Some(true);
                if !open || facts.is_solid_render(ground) != Some(true) {
                    continue;
                }
                if level.brightness(host, LightLayer::Block, pos) != Some(0) {
                    continue;
                }
                let night_only = level.brightness(host, LightLayer::Sky, pos).unwrap_or(0) > 0;
                if night_only && !show_night {
                    continue;
                }
                spots.push((pos, night_only));
                if spots.len() >= MAX_LIGHT_SPOTS {
                    break 'scan;
                }
            }
        }
    }
    state.light_spots = spots;
}

/// Ein flaches Viereck knapp über dem Boden der Stelle `pos`.
fn light_quad(pos: [i32; 3], night_only: bool) -> WorldQuad {
    let [x, y, z] = [f64::from(pos[0]), f64::from(pos[1]) + 0.02, f64::from(pos[2])];
    let (a, b) = (0.1, 0.9);
    let color = if night_only { 0x66FF_D800 } else { 0x66FF_2020 };
    WorldQuad::new(
        [
            [x + a, y, z + a],
            [x + a, y, z + b],
            [x + b, y, z + b],
            [x + b, y, z + a],
        ],
        color,
        WorldDepth::Tested,
    )
}

/// Hat der eigene Spieler gerade den Dunkelheits-Effekt?
fn has_darkness(client: &dyn ClientHost) -> bool {
    let host: &dyn Host = client;
    let (Some(player), Some(key)) = (client.local_player(), host.effect_key("minecraft:darkness"))
    else {
        return false;
    };
    LivingRef::new(player.entity()).effect(host, key).is_some()
}

// ---------------------------------------------------------------- Free Look

/// Augenposition des eigenen Spielers.
fn client_eye(client: &dyn ClientHost) -> Option<[f64; 3]> {
    let host: &dyn Host = client;
    client.local_player()?.entity().eye_position(host)
}

/// Blickrichtung aus Yaw/Pitch in Grad (wie in Minecraft: Yaw 0 = Süden/+Z).
fn look_dir(yaw: f32, pitch: f32) -> [f64; 3] {
    let (yaw, pitch) = (f64::from(yaw).to_radians(), f64::from(pitch).to_radians());
    [-yaw.sin() * pitch.cos(), -pitch.sin(), yaw.cos() * pitch.cos()]
}

/// Jeden Tick: Free Look an/aus und die Mausdrehung vom Spieler auf die Kamera umlegen.
fn free_look_tick(host: &mut dyn ClientHost, state: &mut State) {
    let wanted = state.cfg.get_bool(FREELOOK_ON, true)
        && host.playing()
        && host.mod_screen().is_none()
        && if state.cfg.get_usize(FREELOOK_MODE, 0) == 1 {
            state.freelook_toggled
        } else {
            host.is_down(MOD_ID, KEY_FREELOOK)
        };
    let rotation = {
        let client: &dyn ClientHost = &*host;
        let h: &dyn Host = client;
        client.local_player().and_then(|p| p.entity().rotation(h))
    };
    let third_person = state.cfg.get_f64(FREELOOK_DIST, 4.0) > 0.0;

    let active = state.freelook.is_some();
    if wanted && !active {
        let Some(rot) = rotation else {
            return;
        };
        let mut previous_view = None;
        if third_person {
            previous_view = Some(host.camera_type());
            host.set_camera_type(CameraType::ThirdPersonBack);
        }
        if let Some(eye) = client_eye(&*host) {
            state.eye = [eye, eye];
        }
        state.freelook = Some(FreeLook { player: rot, camera: rot, previous_view });
    } else if wanted {
        let (Some(rot), Some(look)) = (rotation, state.freelook.as_mut()) else {
            return;
        };
        look.camera[0] += rot[0] - look.player[0];
        look.camera[1] = (look.camera[1] + rot[1] - look.player[1]).clamp(-90.0, 90.0);
        let [yaw, pitch] = look.player;
        let _ = host.set_local_rotation(yaw, pitch);
    } else if let Some(look) = state.freelook.take() {
        let [yaw, pitch] = look.player;
        let _ = host.set_local_rotation(yaw, pitch);
        if let Some(view) = look.previous_view {
            host.set_camera_type(view);
        }
        state.freelook_toggled = false;
    }
}

// ---------------------------------------------------------------- Easy Bridge

/// Die waagerechte Richtung (x, z), in die man schaut, auf die nächste Achse gerundet.
fn horizontal_dir(yaw: f32) -> [i32; 2] {
    let yaw = f64::from(yaw).to_radians();
    let (x, z) = (-yaw.sin(), yaw.cos());
    if x.abs() > z.abs() {
        [x.signum() as i32, 0]
    } else {
        [0, z.signum() as i32]
    }
}

/// Das neue Ziel: die Seite des Blocks unter dir (oder an der Kante hinter dir), die nach vorne
/// zeigt, wenn davor Luft ist. Nur beim Blick nach unten und mit einem Block in der Hand.
fn bridge_target(client: &dyn ClientHost, current: Option<&CrosshairTarget>) -> Option<CrosshairTarget> {
    let host: &dyn Host = client;
    let player = client.local_player()?;
    let entity = player.entity();
    let [yaw, pitch] = entity.rotation(host)?;
    if pitch < 25.0 {
        return None;
    }
    let held = held_stack(client)?;
    host.item_block(held.item())?;

    let level = player.level(host)?;
    let facts = host.block_states();
    let open = |p: [i32; 3]| {
        level
            .block_state(host, p)
            .is_none_or(|s| facts.replaceable(s) == Some(true))
    };
    let [dx, dz] = horizontal_dir(yaw);
    let feet = entity.block_position(host)?;
    let mut base = [feet[0], feet[1] - 1, feet[2]];
    if open(base) {
        // Über die Kante hinaus: der Block hinter dir trägt dich.
        base = [base[0] - dx, base[1], base[2] - dz];
        if open(base) {
            return None;
        }
    }
    if !open([base[0] + dx, base[1], base[2] + dz]) {
        return None;
    }
    // Nur ersetzen, wenn das Spiel auf die Oberseite in Fußhöhe, tiefer oder ins Leere zielt.
    match current {
        Some(CrosshairTarget::Block { pos, hit, .. }) => {
            if pos.y > base[1] || (pos.y == base[1] && hit.face != [0, 1, 0]) {
                return None;
            }
        }
        None => {}
        Some(_) => return None,
    }
    let state = level.block_state(host, base)?;
    let point = [
        f64::from(base[0]) + 0.5 + f64::from(dx) * 0.5,
        f64::from(base[1]) + 0.4,
        f64::from(base[2]) + 0.5 + f64::from(dz) * 0.5,
    ];
    Some(CrosshairTarget::Block {
        pos: BlockPos::new(base[0], base[1], base[2]),
        state,
        hit: BlockHit::new(point, [dx, 0, dz], false),
    })
}

/// Der Stack in der Hand (ausgewählter Hotbar-Slot).
fn held_stack(client: &dyn ClientHost) -> Option<ItemStack> {
    let host: &dyn Host = client;
    let player = client.local_player()?;
    let slot = player.selected_slot(host)? as u32;
    player
        .inventory_slots(host)?
        .into_iter()
        .find(|(s, _)| *s == slot)
        .map(|(_, stack)| stack)
}

// ---------------------------------------------------------------- Auto Eat

/// Essen, das man nicht automatisch essen will (Vergiftung, Hunger-Effekt, Teleport).
const BAD_FOOD: [&str; 8] = [
    "minecraft:rotten_flesh",
    "minecraft:spider_eye",
    "minecraft:poisonous_potato",
    "minecraft:pufferfish",
    "minecraft:chicken",
    "minecraft:suspicious_stew",
    "minecraft:chorus_fruit",
    "minecraft:ominous_bottle",
];

/// So lange darf ein Essversuch höchstens dauern (normales Essen braucht 32 Ticks).
const EAT_TIMEOUT: u32 = 60;

fn auto_eat(host: &mut dyn ClientHost, state: &mut State) {
    let on = state.cfg.get_bool(AUTOEAT_ON, false) && host.playing() && host.mod_screen().is_none();
    let food = {
        let client: &dyn ClientHost = &*host;
        let h: &dyn Host = client;
        client.local_player().and_then(|p| p.food(h)).map(|f| f.level)
    };

    if let Some(eating) = &mut state.eating {
        eating.ticks += 1;
        let ate = food.is_some_and(|level| level > eating.start_food);
        let stopped = eating.ticks > 5 && host.local_item_use().is_none();
        if !on || ate || stopped || eating.ticks > EAT_TIMEOUT {
            let previous = eating.previous_slot;
            state.eating = None;
            let still_held = host.mapping_physical_down("key.use") == Ok(Some(true));
            let _ = host.set_mapping_down("key.use", still_held);
            if let Some(slot) = previous {
                let _ = host.select_hotbar_slot(slot);
            }
        } else {
            let _ = host.set_mapping_down("key.use", true);
        }
        return;
    }

    let Some(level) = food else {
        return;
    };
    let hunger = state.cfg.get_f64(AUTOEAT_HUNGER, 14.0).clamp(1.0, 19.0) as i32;
    if !on || level > hunger || host.local_item_use().is_some() {
        return;
    }
    let avoid_bad = state.cfg.get_bool(AUTOEAT_SAFE, true);
    let Some(slot) = best_food_slot(&*host, avoid_bad) else {
        return;
    };
    let current = host
        .local_player()
        .and_then(|p| p.selected_slot(&*host))
        .and_then(|s| u8::try_from(s).ok());
    if Some(slot) != current {
        let _ = host.select_hotbar_slot(slot);
    }
    let _ = host.set_mapping_down("key.use", true);
    state.eating = Some(Eating {
        previous_slot: current.filter(|c| *c != slot),
        ticks: 0,
        start_food: level,
    });
}

/// Der Hotbar-Slot mit dem nahrhaftesten Essen.
fn best_food_slot(client: &dyn ClientHost, avoid_bad: bool) -> Option<u8> {
    let host: &dyn Host = client;
    let slots = client.local_player()?.inventory_slots(host)?;
    let mut best: Option<(u8, f64)> = None;
    for (slot, stack) in slots {
        if slot > 8 {
            continue;
        }
        let Some(food) = component(host, &stack, "minecraft:food") else {
            continue;
        };
        if avoid_bad && BAD_FOOD.contains(&stack.item().id(host).as_str()) {
            continue;
        }
        let nutrition = food.get("nutrition").and_then(number).unwrap_or(0.0);
        let saturation = food.get("saturation").and_then(number).unwrap_or(0.0);
        let value = nutrition + saturation;
        if best.is_none_or(|(_, top)| value > top) {
            best = Some((slot as u8, value));
        }
    }
    best.map(|(slot, _)| slot)
}

// ---------------------------------------------------------------- Auto-Tool

fn auto_tool(host: &mut dyn ClientHost, state: &mut State) {
    if !state.cfg.get_bool(AUTOTOOL_ON, true) {
        state.previous_slot = None;
        return;
    }
    let attacking = host
        .mapping_state("key.attack")
        .is_some_and(|(down, _)| down);
    let block = match host.crosshair_target() {
        Some(CrosshairTarget::Block { state: block, .. }) if attacking => Some(block),
        _ => None,
    };

    let Some(player) = host.local_player() else {
        return;
    };
    let current = player
        .selected_slot(&*host)
        .and_then(|s| u8::try_from(s).ok());

    match block {
        Some(block) => {
            let save_tools = state.cfg.get_bool(AUTOTOOL_SAVE, true);
            let Some(best) = best_hotbar_slot(&*host, player, block, save_tools) else {
                return;
            };
            if Some(best) != current {
                if state.previous_slot.is_none() {
                    state.previous_slot = current;
                }
                let _ = host.select_hotbar_slot(best);
            }
        }
        None => {
            if let Some(previous) = state.previous_slot.take()
                && state.cfg.get_bool(AUTOTOOL_BACK, true)
                && Some(previous) != current
            {
                let _ = host.select_hotbar_slot(previous);
            }
        }
    }
}

/// Der Hotbar-Slot (0..9) mit dem schnellsten passenden Werkzeug für `block`.
fn best_hotbar_slot(
    host: &dyn ClientHost,
    player: PlayerRef,
    block: BlockState,
    save_tools: bool,
) -> Option<u8> {
    let host: &dyn Host = host;
    let slots = player.inventory_slots(host)?;
    let facts = host.block_states();

    let mut best: Option<(u8, f32)> = None;
    for (slot, stack) in slots {
        if slot > 8 {
            continue;
        }
        if facts.is_correct_tool_for_drops(&stack, block) != Some(true) {
            continue;
        }
        if save_tools && remaining_durability(host, &stack).is_some_and(|left| left <= MIN_DURABILITY) {
            continue;
        }
        let speed = mining_speed(host, &stack);
        if best.is_none_or(|(_, top)| speed > top) {
            best = Some((slot as u8, speed));
        }
    }
    best.map(|(slot, _)| slot)
}

fn remaining_durability(host: &dyn Host, stack: &ItemStack) -> Option<u32> {
    let max = host.max_damage(stack.item())?;
    Some(max.saturating_sub(stack.damage()))
}

/// Ein Daten-Komponent des Stacks: eigener Wert, sonst der Standard des Items.
fn component(host: &dyn Host, stack: &ItemStack, id: &str) -> Option<Tag> {
    stack
        .components()
        .and_then(|c| c.as_nbt().get(&id.into()).cloned())
        .or_else(|| host.item_default_component(stack.item(), id))
}

/// Die höchste Abbaugeschwindigkeit aus den `minecraft:tool`-Regeln, für die es das
/// richtige Werkzeug ist (Holz 2, Stein 4, Eisen 6, Diamant 8, Netherit 9, Gold 12).
fn mining_speed(host: &dyn Host, stack: &ItemStack) -> f32 {
    component(host, stack, "minecraft:tool").as_ref().map_or(1.0, tool_speed)
}

fn tool_speed(tool: &Tag) -> f32 {
    let Some(Tag::List(rules)) = tool.get("rules") else {
        return 1.0;
    };
    rules
        .iter()
        .filter(|rule| matches!(rule.get("correct_for_drops"), Some(Tag::Byte(1))))
        .filter_map(|rule| number(rule.get("speed")?))
        .map(|v| v as f32)
        .fold(1.0, f32::max)
}

fn number(tag: &Tag) -> Option<f64> {
    match tag {
        Tag::Float(bits) => Some(f64::from(f32::from_bits(*bits))),
        Tag::Double(bits) => Some(f64::from_bits(*bits)),
        Tag::Int(v) => Some(f64::from(*v)),
        _ => None,
    }
}

// ---------------------------------------------------------------- Autoclicker

fn toggle_clicker(host: &mut dyn ClientHost, state: &mut State) {
    state.clicker_on = !state.clicker_on;
    state.pending_clicks = 0;
    state.ticks_since_click = u32::MAX / 2;
    let line = if state.clicker_on { "Autoclicker on" } else { "Autoclicker off" };
    let _ = host.chat_line(Text::literal(line));
}

fn auto_clicker(host: &mut dyn ClientHost, state: &mut State) {
    state.ticks_since_click = state.ticks_since_click.saturating_add(1);
    if !state.clicker_on {
        return;
    }

    let right = state.cfg.get_usize(CLICKER_BUTTON, 0) == 1;
    let mapping = if right { "key.use" } else { "key.attack" };

    // Links mit „voller Cooldown“: erst schlagen, wenn der Angriff wieder voll aufgeladen ist.
    // Die Ladezeit kommt aus der Angriffsgeschwindigkeit des Items in der Hand
    // (Schwert 1,6/s → 13 Ticks, Axt 0,8–1,0/s → 20–25 Ticks, Hand 4/s → 5 Ticks).
    if !right && state.cfg.get_bool(CLICKER_COOLDOWN, true) {
        let delay = attack_delay_ticks(&*host);
        if state.ticks_since_click >= delay {
            let _ = host.click_mapping(mapping);
            state.ticks_since_click = 0;
        }
        return;
    }

    let cps = state.cfg.get_f64(CLICKER_CPS, 10.0).clamp(1.0, 20.0).round() as u32;
    let clicks = clicks_this_tick(&mut state.pending_clicks, cps);
    for _ in 0..clicks {
        let _ = host.click_mapping(mapping);
        state.ticks_since_click = 0;
    }
}

/// Ticks, bis ein Schlag mit dem Item in der Hand wieder voll geladen ist.
fn attack_delay_ticks(client: &dyn ClientHost) -> u32 {
    let host: &dyn Host = client;
    let held = client.local_player().and_then(|player| {
        let slot = player.selected_slot(host)? as u32;
        player
            .inventory_slots(host)?
            .into_iter()
            .find(|(s, _)| *s == slot)
            .map(|(_, stack)| stack)
    });
    let speed = 4.0
        + held
            .and_then(|stack| component(host, &stack, "minecraft:attribute_modifiers"))
            .map_or(0.0, |tag| attack_speed_bonus(&tag));
    delay_for_speed(speed)
}

/// 20 Ticks pro Sekunde geteilt durch Schläge pro Sekunde, aufgerundet, plus ein Tick Puffer.
fn delay_for_speed(attacks_per_second: f64) -> u32 {
    let speed = attacks_per_second.max(0.1);
    (20.0 / speed).ceil() as u32 + 1
}

/// Summe der `attack_speed`-Werte (`add_value`) für die Haupthand.
fn attack_speed_bonus(tag: &Tag) -> f64 {
    let list = match tag {
        Tag::List(list) => list,
        Tag::Compound(_) => match tag.get("modifiers") {
            Some(Tag::List(list)) => list,
            _ => return 0.0,
        },
        _ => return 0.0,
    };
    list.iter()
        .filter(|m| {
            matches!(m.get("type"), Some(Tag::String(t)) if t.to_string_lossy().ends_with("attack_speed"))
        })
        .filter(|m| match m.get("operation") {
            Some(Tag::String(op)) => op.to_string_lossy() == "add_value",
            None => true,
            _ => false,
        })
        .filter_map(|m| number(m.get("amount")?))
        .sum()
}

/// Verteilt `cps` Klicks gleichmäßig auf 20 Ticks pro Sekunde.
fn clicks_this_tick(pending: &mut u32, cps: u32) -> u32 {
    *pending += cps;
    let whole = *pending / 20;
    *pending %= 20;
    whole
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_cps_setting_gives_exactly_that_many_clicks() {
        for cps in 1..=20 {
            let mut pending = 0;
            let total: u32 = (0..20).map(|_| clicks_this_tick(&mut pending, cps)).sum();
            assert_eq!(total, cps);
        }
    }

    #[test]
    fn cooldown_delays_match_vanilla_weapons() {
        assert_eq!(delay_for_speed(1.6), 14); // Schwert: 12,5 → 13 + 1
        assert_eq!(delay_for_speed(4.0), 6); // Hand: 5 + 1
        assert_eq!(delay_for_speed(1.0), 21); // Diamant-Axt: 20 + 1
    }

    fn compound(entries: Vec<(&str, Tag)>) -> Tag {
        let mut c = Compound::new();
        for (k, v) in entries {
            c.insert(k.into(), v);
        }
        Tag::Compound(c)
    }

    #[test]
    fn sword_attack_speed_is_read_from_its_modifiers() {
        let modifiers = Tag::List(vec![
            compound(vec![
                ("type", Tag::String("minecraft:attack_damage".into())),
                ("amount", Tag::Double(6.0f64.to_bits())),
                ("operation", Tag::String("add_value".into())),
            ]),
            compound(vec![
                ("type", Tag::String("minecraft:attack_speed".into())),
                ("amount", Tag::Double((-2.4f64).to_bits())),
                ("operation", Tag::String("add_value".into())),
            ]),
        ]);
        assert!((attack_speed_bonus(&modifiers) + 2.4).abs() < 1e-9);
    }

    #[test]
    fn tool_speed_takes_the_fastest_correct_rule() {
        let rule = |speed: f32, correct: bool| {
            compound(vec![
                ("speed", Tag::Float(speed.to_bits())),
                ("correct_for_drops", Tag::Byte(correct as i8)),
            ])
        };
        let tool = compound(vec![("rules", Tag::List(vec![rule(1.0, false), rule(8.0, true)]))]);
        assert_eq!(tool_speed(&tool), 8.0);
        assert_eq!(tool_speed(&compound(vec![])), 1.0);
    }

    #[test]
    fn bridge_direction_follows_the_view() {
        assert_eq!(horizontal_dir(0.0), [0, 1]); // Süden
        assert_eq!(horizontal_dir(90.0), [-1, 0]); // Westen
        assert_eq!(horizontal_dir(180.0), [0, -1]); // Norden
        assert_eq!(horizontal_dir(-90.0), [1, 0]); // Osten
    }

    #[test]
    fn weather_view_overrides_only_when_on() {
        WEATHER_MODE.store(0, Ordering::Relaxed);
        assert_eq!(weather_view((0.5, 0.2)), (0.5, 0.2));
        WEATHER_MODE.store(3, Ordering::Relaxed);
        assert_eq!(weather_view((0.0, 0.0)), (1.0, 1.0));
        WEATHER_MODE.store(0, Ordering::Relaxed);
    }

    #[test]
    fn menu_key_choices_are_valid() {
        let mut options = vec!["sunrise_tweaks.menu_key.controls"];
        options.extend(MENU_KEYS.iter().map(|k| *k));
        assert_eq!(SettingSpec::choice(MENU_KEY, "x", options, 0).check(), Ok(()));
    }
}
