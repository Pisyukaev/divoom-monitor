use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::{Path as AxumPath, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use image::codecs::gif::GifEncoder;
use image::Frame;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::divoom_api::send_command;

const HERO_CDN_BASE: &str =
    "https://cdn.cloudflare.steamstatic.com/apps/dota2/images/dota_react/heroes";
const ITEM_CDN_BASE: &str =
    "https://cdn.cloudflare.steamstatic.com/apps/dota2/images/dota_react/items";
const GAME_END_TIMEOUT_SECS: u64 = 30;

// ── Shared state ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeroInfo {
    pub name: String,
    pub display_name: String,
    pub player_name: String,
    pub health: u32,
    pub max_health: u32,
    pub mana: u32,
    pub max_mana: u32,
    pub level: u32,
    pub alive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlayerStats {
    pub kills: u32,
    pub deaths: u32,
    pub assists: u32,
    pub last_hits: u32,
    pub denies: u32,
    pub gold: u32,
    pub gpm: u32,
    pub xpm: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemSlot {
    pub name: String,
    pub charges: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbilitySlot {
    pub name: String,
    pub level: u32,
    pub cooldown: f64,
    pub can_cast: bool,
    pub ultimate: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dota2GameState {
    pub game_active: bool,
    pub heroes: Vec<HeroInfo>,
    pub game_time: Option<f64>,
    pub map_state: Option<String>,
    pub daytime: Option<bool>,
    pub player_stats: Option<PlayerStats>,
    pub items: Vec<ItemSlot>,
    pub abilities: Vec<AbilitySlot>,
}

impl Default for Dota2GameState {
    fn default() -> Self {
        Self {
            game_active: false,
            heroes: Vec::new(),
            game_time: None,
            map_state: None,
            daytime: None,
            player_stats: None,
            items: Vec::new(),
            abilities: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dota2Status {
    pub server_running: bool,
    pub port: u16,
    pub game_state: Dota2GameState,
}

pub struct Dota2Inner {
    pub game_state: Dota2GameState,
    pub device_ip: Option<String>,
    pub local_ip: String,
    pub port: u16,
    pub last_gsi_time: Option<Instant>,
    pub hero_image_cache: HashMap<String, Vec<u8>>,
    pub item_png_cache: HashMap<String, Vec<u8>>,
    pub screens_initialized: bool,
    pub previous_hero_names: Vec<String>,
    pub previous_item_names: Vec<String>,
    pub cache_dir: PathBuf,
    pub server_handle: Option<tokio::task::JoinHandle<()>>,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

pub type SharedDota2State = Arc<RwLock<Dota2Inner>>;

pub fn create_shared_state(cache_dir: PathBuf) -> SharedDota2State {
    Arc::new(RwLock::new(Dota2Inner {
        game_state: Dota2GameState::default(),
        device_ip: None,
        local_ip: detect_local_ip(),
        port: 44444,
        last_gsi_time: None,
        hero_image_cache: HashMap::new(),
        item_png_cache: HashMap::new(),
        screens_initialized: false,
        previous_hero_names: Vec::new(),
        previous_item_names: Vec::new(),
        cache_dir,
        server_handle: None,
        shutdown_tx: None,
    }))
}

// ── HTTP server ──

pub async fn start_server(state: SharedDota2State, port: u16) -> Result<(), String> {
    {
        let inner = state.read().await;
        if inner.server_handle.is_some() {
            return Err("Dota 2 GSI server is already running".into());
        }
    }

    let app = Router::new()
        .route("/", post(handle_gsi_post))
        .route("/text/{lcd_index}/{line}", get(handle_text_request))
        .route("/hero/{hero_name}", get(handle_hero_image))
        .route("/items_composite.gif", get(handle_items_composite))
        .with_state(state.clone());

    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| format!("Failed to bind to port {}: {}", port, e))?;

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    let handle = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await
            .ok();
    });

    let state_clone = state.clone();
    tokio::spawn(async move {
        check_game_end_loop(state_clone).await;
    });

    {
        let mut inner = state.write().await;
        inner.port = port;
        inner.server_handle = Some(handle);
        inner.shutdown_tx = Some(shutdown_tx);
        inner.hero_image_cache.clear();
        let _ = std::fs::remove_dir_all(&inner.cache_dir);
    }

    Ok(())
}

pub async fn stop_server(state: SharedDota2State) {
    let mut inner = state.write().await;
    if let Some(tx) = inner.shutdown_tx.take() {
        let _ = tx.send(());
    }
    if let Some(handle) = inner.server_handle.take() {
        handle.abort();
    }
    inner.game_state = Dota2GameState::default();
    inner.screens_initialized = false;
    inner.previous_hero_names.clear();
    inner.last_gsi_time = None;
}

async fn check_game_end_loop(state: SharedDota2State) {
    loop {
        tokio::time::sleep(Duration::from_secs(5)).await;

        let should_deactivate = {
            let inner = state.read().await;
            if inner.server_handle.is_none() {
                return;
            }
            if !inner.game_state.game_active {
                continue;
            }
            match inner.last_gsi_time {
                Some(t) => t.elapsed() > Duration::from_secs(GAME_END_TIMEOUT_SECS),
                None => false,
            }
        };

        if should_deactivate {
            let mut inner = state.write().await;
            inner.game_state.game_active = false;
            inner.screens_initialized = false;
            inner.previous_hero_names.clear();
            inner.previous_item_names.clear();
            inner.game_state.heroes.clear();
        }
    }
}

// ── Route handlers ──

async fn handle_gsi_post(
    State(state): State<SharedDota2State>,
    Json(payload): Json<serde_json::Value>,
) -> StatusCode {
    let parsed = parse_gsi_payload(&payload);

    let (device_ip, local_ip, port, needs_screen_init, items_changed) = {
        let mut inner = state.write().await;
        inner.last_gsi_time = Some(Instant::now());

        if let Some(ref parsed) = parsed {
            if !parsed.heroes.is_empty() {
                let new_names: Vec<String> =
                    parsed.heroes.iter().map(|h| h.name.clone()).collect();
                let heroes_changed = new_names != inner.previous_hero_names;

                let new_item_names: Vec<String> =
                    parsed.items.iter().map(|i| i.name.clone()).collect();
                let items_did_change = new_item_names != inner.previous_item_names;
                if items_did_change {
                    inner.previous_item_names = new_item_names;
                }

                inner.game_state = parsed.clone();
                inner.game_state.game_active = true;

                let needs_init = heroes_changed || !inner.screens_initialized;
                if heroes_changed {
                    inner.previous_hero_names = new_names;
                    inner.screens_initialized = false;
                }

                (
                    inner.device_ip.clone(),
                    inner.local_ip.clone(),
                    inner.port,
                    needs_init,
                    items_did_change && !needs_init,
                )
            } else {
                return StatusCode::OK;
            }
        } else {
            return StatusCode::OK;
        }
    };

    if needs_screen_init {
        if let Some(ref device_ip) = device_ip {
            let (heroes, is_solo) = {
                let inner = state.read().await;
                let heroes = inner.game_state.heroes.clone();
                (heroes.clone(), heroes.len() == 1)
            };

            if is_solo {
                init_solo_screens(&state, device_ip, &local_ip, port, &heroes[0]).await;
            } else {
                init_team_screens(&state, device_ip, &local_ip, port, &heroes).await;
            }

            let mut inner = state.write().await;
            inner.screens_initialized = true;
        }
    } else if items_changed {
        if let Some(ref device_ip) = device_ip {
            let is_solo = {
                let inner = state.read().await;
                inner.game_state.heroes.len() == 1
            };
            if is_solo {
                update_items_screen(&state, device_ip, &local_ip, port).await;
            }
        }
    }

    StatusCode::OK
}

fn build_text_item(text_id: u32, x: u32, y: u32, url: &str, color: &str, update_time: u32) -> serde_json::Value {
    serde_json::json!({
        "TextId": text_id,
        "type": 23,
        "x": x, "y": y,
        "dir": 0,
        "font": 4,
        "TextWidth": 128,
        "Textheight": 16,
        "TextString": url,
        "speed": 100,
        "color": color,
        "update_time": update_time
    })
}

async fn init_team_screens(
    state: &SharedDota2State,
    device_ip: &str,
    local_ip: &str,
    port: u16,
    heroes: &[HeroInfo],
) {
    for (i, hero) in heroes.iter().enumerate().take(5) {
        let short_name = hero_short_name(&hero.name);
        let _ = ensure_hero_image_cached(state, &short_name).await;

        let image_url = format!("http://{}:{}/hero/{}.gif", local_ip, port, short_name);
        let tb = format!("http://{}:{}/text/{}", local_ip, port, i);
        let bid = (i * 8) as u32;

        let cmd = serde_json::json!({
            "Command": "Draw/SendHttpItemList",
            "LcdIndex": i,
            "NewFlag": 1,
            "BackgroudGif": image_url,
            "ItemList": [
                build_text_item(bid+1, 0, 0,   &format!("{}/name", tb),  "#FFFFFF", 5),
                build_text_item(bid+2, 0, 82,  &format!("{}/level", tb), "#FFD700", 1),
                build_text_item(bid+3, 0, 98,  &format!("{}/hp", tb),    "#4CAF50", 1),
                build_text_item(bid+4, 0, 114, &format!("{}/mana", tb),  "#2196F3", 1),
            ]
        });
        let _ = send_command(device_ip, &cmd).await;
    }
}

async fn init_solo_screens(
    state: &SharedDota2State,
    device_ip: &str,
    local_ip: &str,
    port: u16,
    hero: &HeroInfo,
) {
    let short_name = hero_short_name(&hero.name);
    let _ = ensure_hero_image_cached(state, &short_name).await;
    ensure_dark_bg_cached(state).await;

    let hero_url = format!("http://{}:{}/hero/{}.gif", local_ip, port, short_name);
    let dark_url = format!("http://{}:{}/hero/_dark.gif", local_ip, port);
    let tb = format!("http://{}:{}/text/0", local_ip, port);

    // Screen 0: Hero portrait + nick + level + HP + mana
    let cmd0 = serde_json::json!({
        "Command": "Draw/SendHttpItemList",
        "LcdIndex": 0, "NewFlag": 1,
        "BackgroudGif": hero_url,
        "ItemList": [
            build_text_item(1, 0, 0,   &format!("{}/name", tb),  "#FFFFFF", 5),
            build_text_item(2, 0, 82,  &format!("{}/level", tb), "#FFD700", 1),
            build_text_item(3, 0, 98,  &format!("{}/hp", tb),    "#4CAF50", 1),
            build_text_item(4, 0, 114, &format!("{}/mana", tb),  "#2196F3", 1),
        ]
    });
    let _ = send_command(device_ip, &cmd0).await;

    // Screen 1: Items (composite image)
    generate_and_cache_items_composite(state).await;
    let items_url = format!("http://{}:{}/items_composite.gif", local_ip, port);
    let cmd1 = serde_json::json!({
        "Command": "Draw/SendHttpItemList",
        "LcdIndex": 1, "NewFlag": 1,
        "BackgroudGif": items_url,
        "ItemList": []
    });
    let _ = send_command(device_ip, &cmd1).await;

    // Screen 2: KDA + Gold + GPM/XPM
    let cmd2 = serde_json::json!({
        "Command": "Draw/SendHttpItemList",
        "LcdIndex": 2, "NewFlag": 1,
        "BackgroudGif": dark_url,
        "ItemList": [
            build_text_item(21, 0, 0,  &format!("{}/stats_header", tb), "#FFD700", 60),
            build_text_item(22, 0, 22, &format!("{}/kda", tb),          "#FFFFFF", 1),
            build_text_item(23, 0, 44, &format!("{}/gold", tb),         "#FFD700", 1),
            build_text_item(24, 0, 66, &format!("{}/gpm_xpm", tb),      "#BBBBBB", 2),
            build_text_item(25, 0, 88, &format!("{}/lh_dn", tb),        "#BBBBBB", 2),
        ]
    });
    let _ = send_command(device_ip, &cmd2).await;

    // Screen 3: Abilities
    let cmd3 = serde_json::json!({
        "Command": "Draw/SendHttpItemList",
        "LcdIndex": 3, "NewFlag": 1,
        "BackgroudGif": dark_url,
        "ItemList": [
            build_text_item(31, 0, 0,  &format!("{}/abilities_header", tb), "#FFD700", 60),
            build_text_item(32, 0, 18, &format!("{}/ability_0", tb),        "#FFFFFF", 2),
            build_text_item(33, 0, 34, &format!("{}/ability_1", tb),        "#FFFFFF", 2),
            build_text_item(34, 0, 50, &format!("{}/ability_2", tb),        "#FFFFFF", 2),
            build_text_item(35, 0, 66, &format!("{}/ability_3", tb),        "#FFFFFF", 2),
            build_text_item(36, 0, 82, &format!("{}/ability_4", tb),        "#FFFFFF", 2),
            build_text_item(37, 0, 98, &format!("{}/ability_5", tb),        "#FFFFFF", 2),
        ]
    });
    let _ = send_command(device_ip, &cmd3).await;

    // Screen 4: Game info
    let cmd4 = serde_json::json!({
        "Command": "Draw/SendHttpItemList",
        "LcdIndex": 4, "NewFlag": 1,
        "BackgroudGif": dark_url,
        "ItemList": [
            build_text_item(41, 0, 0,  &format!("{}/game_header", tb),  "#FFD700", 60),
            build_text_item(42, 0, 28, &format!("{}/gametime", tb),     "#FFFFFF", 1),
            build_text_item(43, 0, 56, &format!("{}/daytime", tb),      "#FFFFFF", 5),
            build_text_item(44, 0, 84, &format!("{}/hero_name", tb),    "#BBBBBB", 60),
        ]
    });
    let _ = send_command(device_ip, &cmd4).await;
}

async fn update_items_screen(
    state: &SharedDota2State,
    device_ip: &str,
    local_ip: &str,
    port: u16,
) {
    generate_and_cache_items_composite(state).await;
    let items_url = format!("http://{}:{}/items_composite.gif", local_ip, port);
    let cmd = serde_json::json!({
        "Command": "Draw/SendHttpItemList",
        "LcdIndex": 1, "NewFlag": 1,
        "BackgroudGif": items_url,
        "ItemList": []
    });
    let _ = send_command(device_ip, &cmd).await;
}

async fn ensure_dark_bg_cached(state: &SharedDota2State) {
    {
        let inner = state.read().await;
        if inner.hero_image_cache.contains_key("_dark") {
            return;
        }
    }

    let canvas = image::RgbaImage::from_pixel(128, 128, image::Rgba([15, 15, 20, 255]));
    let mut gif_buf = Vec::new();
    {
        let mut encoder = GifEncoder::new(&mut gif_buf);
        let frame = Frame::new(canvas);
        let _ = encoder.encode_frames(std::iter::once(frame));
    }

    let mut inner = state.write().await;
    inner.hero_image_cache.insert("_dark".to_string(), gif_buf);
}

fn build_text_bar(current: u32, max: u32, bar_len: usize) -> String {
    if max == 0 {
        return "|".repeat(0) + &".".repeat(bar_len);
    }
    let filled = ((current as f64 / max as f64) * bar_len as f64).round() as usize;
    let filled = filled.min(bar_len);
    let empty = bar_len - filled;
    "|".repeat(filled) + &".".repeat(empty)
}

async fn handle_text_request(
    State(state): State<SharedDota2State>,
    AxumPath((lcd_index, line)): AxumPath<(usize, String)>,
) -> Json<serde_json::Value> {
    let inner = state.read().await;
    let gs = &inner.game_state;

    let hero = if lcd_index < gs.heroes.len() {
        Some(&gs.heroes[lcd_index])
    } else if !gs.heroes.is_empty() {
        Some(&gs.heroes[0])
    } else {
        None
    };

    let text = match line.as_str() {
        "name" => hero.map(|h| {
            if h.player_name.is_empty() { h.display_name.clone() } else { h.player_name.clone() }
        }).unwrap_or_default(),
        "level" => hero.map(|h| {
            if h.alive { format!("Lv {}", h.level) } else { format!("Lv {} DEAD", h.level) }
        }).unwrap_or_default(),
        "hp" => hero.map(|h| {
            let bar = build_text_bar(h.health, h.max_health, 10);
            format!("HP {} {}/{}", bar, h.health, h.max_health)
        }).unwrap_or_default(),
        "mana" => hero.map(|h| {
            let bar = build_text_bar(h.mana, h.max_mana, 10);
            format!("MP {} {}/{}", bar, h.mana, h.max_mana)
        }).unwrap_or_default(),

        "items_header" => "-- ITEMS --".to_string(),
        l if l.starts_with("item_") => {
            if let Ok(idx) = l[5..].parse::<usize>() {
                gs.items.get(idx).map(|item| {
                    let name = item_display_name(&item.name);
                    if item.charges > 0 { format!("{} ({})", name, item.charges) } else { name }
                }).unwrap_or_else(|| "---".to_string())
            } else { String::new() }
        }

        "stats_header" => "-- STATS --".to_string(),
        "kda" => gs.player_stats.as_ref().map(|s|
            format!("K {} / D {} / A {}", s.kills, s.deaths, s.assists)
        ).unwrap_or_default(),
        "gold" => gs.player_stats.as_ref().map(|s|
            format!("Gold: {}", s.gold)
        ).unwrap_or_default(),
        "gpm_xpm" => gs.player_stats.as_ref().map(|s|
            format!("GPM {} XPM {}", s.gpm, s.xpm)
        ).unwrap_or_default(),
        "lh_dn" => gs.player_stats.as_ref().map(|s|
            format!("LH {} / DN {}", s.last_hits, s.denies)
        ).unwrap_or_default(),

        "abilities_header" => "-- ABILITIES --".to_string(),
        l if l.starts_with("ability_") => {
            if let Ok(idx) = l[8..].parse::<usize>() {
                gs.abilities.get(idx).map(|a| {
                    let name = ability_display_name(&a.name);
                    if a.cooldown > 0.0 {
                        format!("[{}] {} CD:{:.0}s", a.level, name, a.cooldown)
                    } else {
                        format!("[{}] {}", a.level, name)
                    }
                }).unwrap_or_else(|| "---".to_string())
            } else { String::new() }
        }

        "game_header" => "-- GAME --".to_string(),
        "gametime" => {
            if let Some(t) = gs.game_time {
                let secs = t as i64;
                let m = secs / 60;
                let s = (secs % 60).abs();
                format!("Time: {}:{:02}", m, s)
            } else { "Time: --:--".to_string() }
        }
        "daytime" => {
            match gs.daytime {
                Some(true) => "Day".to_string(),
                Some(false) => "Night".to_string(),
                None => String::new(),
            }
        }
        "hero_name" => hero.map(|h| h.display_name.clone()).unwrap_or_default(),

        _ => String::new(),
    };

    Json(serde_json::json!({ "DispData": text }))
}

async fn handle_hero_image(
    State(state): State<SharedDota2State>,
    AxumPath(hero_name): AxumPath<String>,
) -> Response {
    let name = hero_name.trim_end_matches(".gif");

    if let Err(_) = ensure_hero_image_cached(&state, name).await {
        return (StatusCode::NOT_FOUND, "Hero image not found").into_response();
    }

    let inner = state.read().await;
    if let Some(data) = inner.hero_image_cache.get(name) {
        (
            StatusCode::OK,
            [("content-type", "image/gif")],
            data.clone(),
        )
            .into_response()
    } else {
        (StatusCode::NOT_FOUND, "Hero image not found").into_response()
    }
}

// ── GSI JSON parsing ──

fn parse_gsi_payload(payload: &serde_json::Value) -> Option<Dota2GameState> {
    let map_state = payload
        .get("map")
        .and_then(|m| m.get("game_state"))
        .and_then(|s| s.as_str())
        .map(|s| s.to_string());

    let game_time = payload
        .get("map")
        .and_then(|m| m.get("clock_time"))
        .and_then(|t| t.as_f64());

    let is_spectating = payload
        .get("player")
        .and_then(|p| p.get("team2"))
        .is_some();

    let mut heroes = Vec::new();

    if is_spectating {
        let player_team = determine_player_team_spectator(payload);
        let team_key = match player_team.as_deref() {
            Some("radiant") => "team2",
            Some("dire") => "team3",
            _ => "team2",
        };

        if let Some(hero_team) = payload.get("hero").and_then(|h| h.get(team_key)) {
            let player_section = payload.get("player").and_then(|p| p.get(team_key));
            for i in 0..5 {
                let player_key = format!("player{}", if team_key == "team3" { i + 5 } else { i });
                if let Some(hero_data) = hero_team.get(&player_key) {
                    let player_name = player_section
                        .and_then(|ps| ps.get(&player_key))
                        .and_then(|p| p.get("name"))
                        .and_then(|n| n.as_str())
                        .unwrap_or("")
                        .to_string();
                    if let Some(mut info) = parse_hero_data(hero_data) {
                        info.player_name = player_name;
                        heroes.push(info);
                    }
                }
            }
        }
    } else {
        if let Some(hero_data) = payload.get("hero") {
            if hero_data.get("name").is_some() {
                let player_name = payload
                    .get("player")
                    .and_then(|p| p.get("name"))
                    .and_then(|n| n.as_str())
                    .unwrap_or("")
                    .to_string();
                if let Some(mut info) = parse_hero_data(hero_data) {
                    info.player_name = player_name;
                    heroes.push(info);
                }
            }
        }
    }

    let daytime = payload
        .get("map")
        .and_then(|m| m.get("daytime"))
        .and_then(|d| d.as_bool());

    let player_stats = if !is_spectating {
        payload.get("player").map(|p| PlayerStats {
            kills: p.get("kills").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            deaths: p.get("deaths").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            assists: p.get("assists").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            last_hits: p.get("last_hits").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            denies: p.get("denies").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            gold: p.get("gold").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            gpm: p.get("gpm").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            xpm: p.get("xpm").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        })
    } else {
        None
    };

    let items = if !is_spectating {
        parse_items(payload)
    } else {
        Vec::new()
    };

    let abilities = if !is_spectating {
        parse_abilities(payload)
    } else {
        Vec::new()
    };

    if heroes.is_empty() && map_state.is_none() {
        return None;
    }

    Some(Dota2GameState {
        game_active: true,
        heroes,
        game_time,
        map_state,
        daytime,
        player_stats,
        items,
        abilities,
    })
}

fn determine_player_team_spectator(payload: &serde_json::Value) -> Option<String> {
    if let Some(player) = payload.get("player") {
        if player.get("team2").is_some() {
            return Some("radiant".to_string());
        }
    }
    Some("radiant".to_string())
}

fn parse_hero_data(data: &serde_json::Value) -> Option<HeroInfo> {
    let name = data.get("name").and_then(|n| n.as_str())?.to_string();
    let display_name = hero_display_name(&name);

    Some(HeroInfo {
        name: name.clone(),
        display_name,
        player_name: String::new(),
        health: data.get("health").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        max_health: data
            .get("max_health")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32,
        mana: data.get("mana").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        max_mana: data
            .get("max_mana")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32,
        level: data.get("level").and_then(|v| v.as_u64()).unwrap_or(1) as u32,
        alive: data.get("alive").and_then(|v| v.as_bool()).unwrap_or(true),
    })
}

fn parse_items(payload: &serde_json::Value) -> Vec<ItemSlot> {
    let mut items = Vec::new();
    if let Some(items_obj) = payload.get("items") {
        for i in 0..6 {
            let key = format!("slot{}", i);
            if let Some(slot) = items_obj.get(&key) {
                let name = slot.get("name").and_then(|n| n.as_str()).unwrap_or("empty").to_string();
                let charges = slot.get("charges").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                items.push(ItemSlot { name, charges });
            } else {
                items.push(ItemSlot { name: "empty".to_string(), charges: 0 });
            }
        }
    }
    items
}

fn parse_abilities(payload: &serde_json::Value) -> Vec<AbilitySlot> {
    let mut abilities = Vec::new();
    if let Some(abs_obj) = payload.get("abilities") {
        for i in 0..6 {
            let key = format!("ability{}", i);
            if let Some(ab) = abs_obj.get(&key) {
                let name = ab.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string();
                if name.is_empty() || name.starts_with("generic_hidden") {
                    continue;
                }
                abilities.push(AbilitySlot {
                    name,
                    level: ab.get("level").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                    cooldown: ab.get("cooldown").and_then(|v| v.as_f64()).unwrap_or(0.0),
                    can_cast: ab.get("can_cast").and_then(|v| v.as_bool()).unwrap_or(false),
                    ultimate: ab.get("ultimate").and_then(|v| v.as_bool()).unwrap_or(false),
                });
            }
        }
    }
    abilities
}

fn item_display_name(raw: &str) -> String {
    let short = raw.strip_prefix("item_").unwrap_or(raw);
    if short == "empty" {
        return "---".to_string();
    }
    short
        .split('_')
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(ch) => ch.to_uppercase().to_string() + c.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn ability_display_name(raw: &str) -> String {
    let parts: Vec<&str> = raw.splitn(2, '_').collect();
    let meaningful = if parts.len() > 1 { parts[1] } else { raw };
    meaningful
        .split('_')
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(ch) => ch.to_uppercase().to_string() + c.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

// ── Hero image handling ──

async fn ensure_hero_image_cached(state: &SharedDota2State, short_name: &str) -> Result<(), String> {
    {
        let inner = state.read().await;
        if inner.hero_image_cache.contains_key(short_name) {
            return Ok(());
        }
    }

    let cache_dir = {
        let inner = state.read().await;
        inner.cache_dir.clone()
    };

    let file_path = cache_dir.join(format!("{}.gif", short_name));
    if file_path.exists() {
        if let Ok(data) = std::fs::read(&file_path) {
            let mut inner = state.write().await;
            inner.hero_image_cache.insert(short_name.to_string(), data);
            return Ok(());
        }
    }

    let png_url = format!("{}/{}.png", HERO_CDN_BASE, short_name);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let response = client
        .get(&png_url)
        .send()
        .await
        .map_err(|e| format!("Failed to download hero image: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Hero image download failed: {}",
            response.status()
        ));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read image bytes: {}", e))?;

    let img = image::load_from_memory(&bytes)
        .map_err(|e| format!("Failed to decode hero image: {}", e))?;

    let resized = img.resize(128, 128, image::imageops::FilterType::Lanczos3);
    let mut canvas = image::RgbaImage::new(128, 128);
    let offset_x = (128 - resized.width()) / 2;
    let offset_y = (128 - resized.height()) / 2;
    image::imageops::overlay(&mut canvas, &resized.to_rgba8(), offset_x as i64, offset_y as i64);
    let rgba = canvas;

    let mut gif_buf = Vec::new();
    {
        let mut encoder = GifEncoder::new(&mut gif_buf);
        let frame = Frame::new(rgba);
        encoder
            .encode_frames(std::iter::once(frame))
            .map_err(|e| format!("GIF encoding error: {}", e))?;
    }

    let _ = std::fs::create_dir_all(&cache_dir);
    let _ = std::fs::write(&file_path, &gif_buf);

    let mut inner = state.write().await;
    inner
        .hero_image_cache
        .insert(short_name.to_string(), gif_buf);

    Ok(())
}

// ── Item image handling ──

async fn ensure_item_image_cached(state: &SharedDota2State, item_name: &str) -> Result<(), String> {
    let short = item_name.strip_prefix("item_").unwrap_or(item_name);
    if short == "empty" || short.is_empty() {
        return Ok(());
    }

    {
        let inner = state.read().await;
        if inner.item_png_cache.contains_key(short) {
            return Ok(());
        }
    }

    let png_url = format!("{}/{}.png", ITEM_CDN_BASE, short);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let response = client
        .get(&png_url)
        .send()
        .await
        .map_err(|e| format!("Failed to download item image: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Item image download failed: {}", response.status()));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read item image bytes: {}", e))?;

    let mut inner = state.write().await;
    inner.item_png_cache.insert(short.to_string(), bytes.to_vec());
    Ok(())
}

async fn generate_and_cache_items_composite(state: &SharedDota2State) {
    let items = {
        let inner = state.read().await;
        inner.game_state.items.clone()
    };

    for item in &items {
        let _ = ensure_item_image_cached(state, &item.name).await;
    }

    let gif_buf = {
        let inner = state.read().await;

        let mut canvas = image::RgbaImage::from_pixel(128, 128, image::Rgba([15, 15, 20, 255]));

        let cols = 3u32;
        let rows = 2u32;
        let cell_w = 42u32;
        let cell_h = 60u32;
        let gap_x = (128 - cell_w * cols) / (cols - 1);
        let gap_y = (128 - cell_h * rows) / (rows + 1);

        for (idx, item) in items.iter().enumerate().take(6) {
            let short = item.name.strip_prefix("item_").unwrap_or(&item.name);
            if short == "empty" || short.is_empty() {
                continue;
            }

            let col = (idx as u32) % cols;
            let row = (idx as u32) / cols;
            let cell_x = col * (cell_w + gap_x);
            let cell_y = gap_y + row * (cell_h + gap_y);

            if let Some(png_bytes) = inner.item_png_cache.get(short) {
                if let Ok(img) = image::load_from_memory(png_bytes) {
                    let resized = img.resize(cell_w, cell_h, image::imageops::FilterType::Lanczos3);
                    let rx = cell_x + (cell_w.saturating_sub(resized.width())) / 2;
                    let ry = cell_y + (cell_h.saturating_sub(resized.height())) / 2;
                    image::imageops::overlay(&mut canvas, &resized.to_rgba8(), rx as i64, ry as i64);
                }
            }
        }

        let mut buf = Vec::new();
        {
            let mut encoder = GifEncoder::new(&mut buf);
            let frame = Frame::new(canvas);
            let _ = encoder.encode_frames(std::iter::once(frame));
        }
        buf
    };

    let mut inner = state.write().await;
    inner.hero_image_cache.insert("_items_composite".to_string(), gif_buf);
}

async fn handle_items_composite(
    State(state): State<SharedDota2State>,
) -> Response {
    let inner = state.read().await;
    if let Some(data) = inner.hero_image_cache.get("_items_composite") {
        (
            StatusCode::OK,
            [("content-type", "image/gif")],
            data.clone(),
        )
            .into_response()
    } else {
        drop(inner);
        ensure_dark_bg_cached(&state).await;
        let inner = state.read().await;
        if let Some(data) = inner.hero_image_cache.get("_dark") {
            (
                StatusCode::OK,
                [("content-type", "image/gif")],
                data.clone(),
            )
                .into_response()
        } else {
            (StatusCode::NOT_FOUND, "Items composite not ready").into_response()
        }
    }
}

// ── Helper functions ──

fn hero_short_name(full_name: &str) -> String {
    full_name
        .strip_prefix("npc_dota_hero_")
        .unwrap_or(full_name)
        .to_string()
}

fn hero_display_name(full_name: &str) -> String {
    let short = hero_short_name(full_name);
    short
        .split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(c) => c.to_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn detect_local_ip() -> String {
    if let Ok(addrs) = std::net::UdpSocket::bind("0.0.0.0:0") {
        if addrs.connect("8.8.8.8:80").is_ok() {
            if let Ok(local_addr) = addrs.local_addr() {
                return local_addr.ip().to_string();
            }
        }
    }
    "127.0.0.1".to_string()
}

// ── Dota 2 path detection ──

pub fn detect_dota2_install_path() -> Option<String> {
    let steam_paths = get_steam_library_paths();

    for lib_path in &steam_paths {
        let dota_path = Path::new(lib_path)
            .join("steamapps")
            .join("common")
            .join("dota 2 beta");
        if dota_path.exists() {
            return Some(dota_path.to_string_lossy().to_string());
        }
    }

    None
}

fn get_steam_library_paths() -> Vec<String> {
    let mut paths = Vec::new();

    let default_paths = [
        r"C:\Program Files (x86)\Steam",
        r"C:\Program Files\Steam",
        r"D:\Steam",
        r"D:\SteamLibrary",
        r"E:\SteamLibrary",
    ];

    for p in &default_paths {
        if Path::new(p).exists() {
            paths.push(p.to_string());
        }
    }

    let default_steam = Path::new(r"C:\Program Files (x86)\Steam");
    let vdf_path = default_steam
        .join("steamapps")
        .join("libraryfolders.vdf");

    if let Ok(content) = std::fs::read_to_string(&vdf_path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("\"path\"") {
                if let Some(path_value) = trimmed.split('"').nth(3) {
                    let normalized = path_value.replace("\\\\", "\\");
                    if Path::new(&normalized).exists() && !paths.contains(&normalized) {
                        paths.push(normalized);
                    }
                }
            }
        }
    }

    paths
}

pub fn setup_gsi_config(dota_path: &str, port: u16) -> Result<String, String> {
    let cfg_dir = Path::new(dota_path)
        .join("game")
        .join("dota")
        .join("cfg")
        .join("gamestate_integration");

    std::fs::create_dir_all(&cfg_dir)
        .map_err(|e| format!("Failed to create GSI config directory: {}", e))?;

    let cfg_file = cfg_dir.join("gamestate_integration_divoom.cfg");

    let config_content = format!(
        r#""Divoom Monitor GSI"
{{
    "uri"           "http://localhost:{}/"
    "timeout"       "5.0"
    "buffer"        "0.1"
    "throttle"      "0.5"
    "heartbeat"     "30.0"
    "data"
    {{
        "provider"      "1"
        "map"           "1"
        "player"        "1"
        "hero"          "1"
        "abilities"     "1"
        "items"         "1"
    }}
}}
"#,
        port
    );

    std::fs::write(&cfg_file, config_content)
        .map_err(|e| format!("Failed to write GSI config file: {}", e))?;

    Ok(cfg_file.to_string_lossy().to_string())
}
