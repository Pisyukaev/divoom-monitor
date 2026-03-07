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
    pub radiant_score: Option<u32>,
    pub dire_score: Option<u32>,
    pub buyback_cost: Option<u32>,
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
            radiant_score: None,
            dire_score: None,
            buyback_cost: None,
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

    let (device_ip, local_ip, port, needs_screen_init) = {
        let mut inner = state.write().await;
        inner.last_gsi_time = Some(Instant::now());

        if let Some(ref parsed) = parsed {
            if !parsed.heroes.is_empty() {
                let new_names: Vec<String> = parsed.heroes.iter().map(|h| h.name.clone()).collect();
                let heroes_changed = new_names != inner.previous_hero_names;

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
    }

    StatusCode::OK
}

fn build_text_item(
    text_id: u32,
    font: u32,
    text_width: u32,
    x: u32,
    y: u32,
    url: &str,
    color: &str,
    update_time: u32,
) -> serde_json::Value {
    serde_json::json!({
        "TextId": text_id,
        "type": 23,
        "x": x,
        "y": y,
        "dir": 0,
        "font": font,
        "TextWidth": text_width,
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
                build_text_item(bid+1, 4, 128, 0, 0,   &format!("{}/name", tb),  "#FFFFFF", 5),
                build_text_item(bid+2, 4, 128, 0, 82,  &format!("{}/level", tb), "#FFD700", 1),
                build_text_item(bid+3, 4, 128, 0, 98,  &format!("{}/hp", tb),    "#4CAF50", 1),
                build_text_item(bid+4, 4, 128, 0, 114, &format!("{}/mana", tb),  "#2196F3", 1),
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
    generate_gold_bg(state).await;
    generate_kda_bg(state).await;
    ensure_faction_bg_cached(state, "radiant").await;
    ensure_faction_bg_cached(state, "dire").await;

    let hero_url = format!("http://{}:{}/hero/{}.gif", local_ip, port, short_name);
    let gold_url = format!("http://{}:{}/hero/_gold_bg.gif", local_ip, port);
    let kda_url = format!("http://{}:{}/hero/_kda_bg.gif", local_ip, port);
    let radiant_url = format!("http://{}:{}/hero/_faction_radiant.gif", local_ip, port);
    let dire_url = format!("http://{}:{}/hero/_faction_dire.gif", local_ip, port);
    let tb = format!("http://{}:{}/text/0", local_ip, port);

    // Screen 0: Hero portrait + level + HP + mana
    let cmd0 = serde_json::json!({
        "Command": "Draw/SendHttpItemList",
        "LcdIndex": 0,
        "NewFlag": 1,
        "BackgroudGif": hero_url,
        "ItemList": [
            build_text_item(1, 4, 128, 0, 0,   &format!("{}/name", tb),  "#FFFFFF", 5),
            build_text_item(2, 4, 128, 0, 82,  &format!("{}/level", tb), "#FFD700", 1),
            build_text_item(3, 4, 128, 0, 98,  &format!("{}/hp", tb),    "#4CAF50", 1),
            build_text_item(4, 4, 128, 0, 114, &format!("{}/mana", tb),  "#2196F3", 1),
        ]
    });
    let _ = send_command(device_ip, &cmd0).await;

    // Screen 1: Gold + Buyback cost
    let cmd1 = serde_json::json!({
        "Command": "Draw/SendHttpItemList",
        "LcdIndex": 1,
        "NewFlag": 1,
        "BackgroudGif": gold_url,
        "ItemList": [
            build_text_item(11, 4, 128, 48, 20, &format!("{}/gold_label", tb),  "#FFFFFF", 60),
            build_text_item(12, 24, 128, 32, 56, &format!("{}/gold_amount", tb), "#FFD700", 1),
        ]
    });
    let _ = send_command(device_ip, &cmd1).await;

    // Screen 2: KDA
    let cmd2 = serde_json::json!({
        "Command": "Draw/SendHttpItemList",
        "LcdIndex": 2,
        "NewFlag": 1,
        "BackgroudGif": kda_url,
        "ItemList": [
            build_text_item(21, 4, 40, 15, 90, &format!("{}/kills_line", tb),   "#FFFFFF", 1),
            build_text_item(22, 4, 40, 60, 90, &format!("{}/deaths_line", tb),  "#FFFFFF", 1),
            build_text_item(23, 4, 40, 100, 90, &format!("{}/assists_line", tb), "#FFFFFF", 1),
        ]
    });
    let _ = send_command(device_ip, &cmd2).await;

    // Screen 3: Radiant kills
    let cmd3 = serde_json::json!({
    "Command": "Draw/SendHttpItemList",
    "LcdIndex": 3,
    "NewFlag": 1,
    "BackgroudGif": radiant_url,
    "ItemList": [
        build_text_item(31, 4, 128, 48, 10,  &format!("{}/radiant_label", tb), "#66BB6A", 60),
        build_text_item(32, 24,128, 50, 64,  &format!("{}/radiant_score", tb), "#4CAF50", 1),
        ]
    });
    let _ = send_command(device_ip, &cmd3).await;

    // Screen 4: Dire kills
    let cmd4 = serde_json::json!({
        "Command": "Draw/SendHttpItemList",
        "LcdIndex": 4,
        "NewFlag": 1,
        "BackgroudGif": dire_url,
        "ItemList": [
            build_text_item(31, 4, 128, 48, 10,  &format!("{}/dire_label", tb), "#EF5350", 60),
            build_text_item(32, 24, 128, 50, 64,  &format!("{}/dire_score", tb), "#F44336", 1),
        ]
    });
    let _ = send_command(device_ip, &cmd4).await;
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

fn draw_filled_circle(
    canvas: &mut image::RgbaImage,
    cx: i32,
    cy: i32,
    r: i32,
    color: image::Rgba<u8>,
) {
    for y in (cy - r)..=(cy + r) {
        for x in (cx - r)..=(cx + r) {
            let dx = x - cx;
            let dy = y - cy;
            if dx * dx + dy * dy <= r * r {
                if x >= 0 && x < 128 && y >= 0 && y < 128 {
                    canvas.put_pixel(x as u32, y as u32, color);
                }
            }
        }
    }
}

fn rgba_to_gif(canvas: image::RgbaImage) -> Vec<u8> {
    let mut gif_buf = Vec::new();
    {
        let mut encoder = GifEncoder::new(&mut gif_buf);
        let frame = Frame::new(canvas);
        let _ = encoder.encode_frames(std::iter::once(frame));
    }
    gif_buf
}

async fn generate_gold_bg(state: &SharedDota2State) {
    {
        let inner = state.read().await;
        if inner.hero_image_cache.contains_key("_gold_bg") {
            return;
        }
    }

    let gold_image = image::load_from_memory(include_bytes!("assets/dota2/gold.webp")).unwrap();
    let resized = gold_image.resize(32, 32, image::imageops::FilterType::Lanczos3);

    let mut canvas = image::RgbaImage::new(128, 128);

    let offset_x = (128 - resized.width()) / 2 + resized.width();
    let offset_y = (128 - resized.height()) / 2;

    image::imageops::overlay(&mut canvas, &resized, offset_x as i64, offset_y as i64);

    let gif_buf = rgba_to_gif(canvas);

    let mut inner = state.write().await;
    inner
        .hero_image_cache
        .insert("_gold_bg".to_string(), gif_buf);
}

async fn generate_kda_bg(state: &SharedDota2State) {
    {
        let inner = state.read().await;
        if inner.hero_image_cache.contains_key("_kda_bg") {
            return;
        }
    }

    let kda_image = image::load_from_memory(include_bytes!("assets/dota2/kda.png")).unwrap();
    let resized = kda_image.resize(128, 128, image::imageops::FilterType::Lanczos3);

    let mut canvas = image::RgbaImage::new(128, 128);

    let offset_x = (128 - resized.width()) / 2;
    let offset_y = (128 - resized.height()) / 2;

    image::imageops::overlay(&mut canvas, &resized, offset_x as i64, offset_y as i64);

    let gif_buf = rgba_to_gif(canvas);

    let mut inner = state.write().await;
    inner
        .hero_image_cache
        .insert("_kda_bg".to_string(), gif_buf);
}

async fn ensure_faction_bg_cached(state: &SharedDota2State, faction: &str) {
    let cache_key = format!("_faction_{}", faction);
    {
        let inner = state.read().await;
        if inner.hero_image_cache.contains_key(&cache_key) {
            return;
        }
    }

    let bg_color = match faction {
        "radiant" => image::Rgba([10, 30, 15, 255]),
        "dire" => image::Rgba([30, 10, 10, 255]),
        _ => image::Rgba([15, 15, 20, 255]),
    };

    let mut canvas = image::RgbaImage::from_pixel(128, 128, bg_color);

    let icon_url = format!(
        "https://dota2.fandom.com/api.php?action=query&titles=File:{}_icon.png&prop=imageinfo&iiprop=url&format=json",
        capitalize_first(faction)
    );

    let icon_loaded = 'download: {
        let client = match reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
        {
            Ok(c) => c,
            Err(_) => break 'download false,
        };

        let api_resp = match client.get(&icon_url).send().await {
            Ok(r) if r.status().is_success() => match r.text().await {
                Ok(t) => t,
                Err(_) => break 'download false,
            },
            _ => break 'download false,
        };

        let direct_url = extract_wiki_image_url(&api_resp);
        let direct_url = match direct_url {
            Some(u) => u,
            None => break 'download false,
        };

        let img_resp = match client.get(&direct_url).send().await {
            Ok(r) if r.status().is_success() => match r.bytes().await {
                Ok(b) => b,
                Err(_) => break 'download false,
            },
            _ => break 'download false,
        };

        if let Ok(img) = image::load_from_memory(&img_resp) {
            let rgba = img.to_rgba8();

            let ox = (128 - rgba.width()) / 2;
            let oy = (128 - rgba.height()) / 2;

            for py in 0..rgba.height() {
                for px in 0..rgba.width() {
                    let p = rgba.get_pixel(px, py);
                    if p[3] > 20 {
                        let dst_x = ox + px;
                        let dst_y = oy + py;
                        if dst_x < 128 && dst_y < 128 {
                            let alpha = (p[3] as f64 * 0.4) as u8;
                            let bg = canvas.get_pixel(dst_x, dst_y);
                            let a = alpha as f64 / 255.0;
                            let r = (p[0] as f64 * a + bg[0] as f64 * (1.0 - a)) as u8;
                            let g = (p[1] as f64 * a + bg[1] as f64 * (1.0 - a)) as u8;
                            let b = (p[2] as f64 * a + bg[2] as f64 * (1.0 - a)) as u8;
                            canvas.put_pixel(dst_x, dst_y, image::Rgba([r, g, b, 255]));
                        }
                    }
                }
            }
            true
        } else {
            false
        }
    };

    if !icon_loaded {
        // Fallback: draw a simple colored symbol
        match faction {
            "radiant" => {
                let color = image::Rgba([30, 80, 40, 255]);
                // Sun-burst pattern
                for angle in 0..12 {
                    let a = (angle as f64) * std::f64::consts::PI / 6.0;
                    for r in 15..45i32 {
                        let x = 64 + (r as f64 * a.cos()) as i32;
                        let y = 64 + (r as f64 * a.sin()) as i32;
                        if x >= 0 && x < 128 && y >= 0 && y < 128 {
                            canvas.put_pixel(x as u32, y as u32, color);
                        }
                    }
                }
                draw_filled_circle(&mut canvas, 64, 64, 15, image::Rgba([25, 65, 35, 255]));
            }
            "dire" => {
                let color = image::Rgba([80, 25, 25, 255]);
                draw_filled_circle(&mut canvas, 64, 64, 30, color);
                draw_filled_circle(&mut canvas, 64, 64, 20, image::Rgba([50, 15, 15, 255]));
                // "Eyes" in the circle
                draw_filled_circle(&mut canvas, 54, 58, 5, image::Rgba([100, 30, 30, 255]));
                draw_filled_circle(&mut canvas, 74, 58, 5, image::Rgba([100, 30, 30, 255]));
            }
            _ => {}
        }
    }

    let gif_buf = rgba_to_gif(canvas);

    let mut inner = state.write().await;
    inner.hero_image_cache.insert(cache_key, gif_buf);
}

fn capitalize_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(ch) => ch.to_uppercase().to_string() + c.as_str(),
        None => String::new(),
    }
}

fn extract_wiki_image_url(json_text: &str) -> Option<String> {
    let parsed: serde_json::Value = serde_json::from_str(json_text).ok()?;
    let pages = parsed.get("query")?.get("pages")?.as_object()?;
    for (_id, page) in pages {
        if let Some(imageinfo) = page.get("imageinfo") {
            if let Some(arr) = imageinfo.as_array() {
                if let Some(first) = arr.first() {
                    return first
                        .get("url")
                        .and_then(|u| u.as_str())
                        .map(|s| s.to_string());
                }
            }
        }
    }
    None
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
        "name" => hero
            .map(|h| {
                if h.player_name.is_empty() {
                    h.display_name.clone()
                } else {
                    h.player_name.clone()
                }
            })
            .unwrap_or_default(),
        "level" => hero
            .map(|h| {
                if h.alive {
                    format!("Lv {}", h.level)
                } else {
                    format!("Lv {} DEAD", h.level)
                }
            })
            .unwrap_or_default(),
        "hp" => hero
            .map(|h| {
                let bar = build_text_bar(h.health, h.max_health, 10);
                format!("HP {} {}/{}", bar, h.health, h.max_health)
            })
            .unwrap_or_default(),
        "mana" => hero
            .map(|h| {
                let bar = build_text_bar(h.mana, h.max_mana, 10);
                format!("MP {} {}/{}", bar, h.mana, h.max_mana)
            })
            .unwrap_or_default(),

        "items_header" => "-- ITEMS --".to_string(),
        l if l.starts_with("item_") => {
            if let Ok(idx) = l[5..].parse::<usize>() {
                gs.items
                    .get(idx)
                    .map(|item| {
                        let name = item_display_name(&item.name);
                        if item.charges > 0 {
                            format!("{} ({})", name, item.charges)
                        } else {
                            name
                        }
                    })
                    .unwrap_or_else(|| "---".to_string())
            } else {
                String::new()
            }
        }

        "gold_label" => "GOLD".to_string(),
        "gold_amount" => gs
            .player_stats
            .as_ref()
            .map(|s| format!("{}", s.gold))
            .unwrap_or_else(|| "0".to_string()),
        "buyback_label" => "BUYBACK".to_string(),
        "buyback" => gs
            .buyback_cost
            .map(|c| format!("{}", c))
            .unwrap_or_else(|| "---".to_string()),

        "kills_line" => gs
            .player_stats
            .as_ref()
            .map(|s| format!("{}", s.kills))
            .unwrap_or_else(|| "0".to_string()),
        "deaths_line" => gs
            .player_stats
            .as_ref()
            .map(|s| format!("{}", s.deaths))
            .unwrap_or_else(|| "0".to_string()),
        "assists_line" => gs
            .player_stats
            .as_ref()
            .map(|s| format!("{}", s.assists))
            .unwrap_or_else(|| "0".to_string()),

        "radiant_label" => "RADIANT".to_string(),
        "radiant_score" => gs
            .radiant_score
            .map(|s| s.to_string())
            .unwrap_or_else(|| "0".to_string()),
        "dire_label" => "DIRE".to_string(),
        "dire_score" => gs
            .dire_score
            .map(|s| s.to_string())
            .unwrap_or_else(|| "0".to_string()),

        "stats_header" => "-- STATS --".to_string(),
        "kda" => gs
            .player_stats
            .as_ref()
            .map(|s| format!("K {} / D {} / A {}", s.kills, s.deaths, s.assists))
            .unwrap_or_default(),
        "gold" => gs
            .player_stats
            .as_ref()
            .map(|s| format!("Gold: {}", s.gold))
            .unwrap_or_default(),
        "gpm_xpm" => gs
            .player_stats
            .as_ref()
            .map(|s| format!("GPM {} XPM {}", s.gpm, s.xpm))
            .unwrap_or_default(),
        "lh_dn" => gs
            .player_stats
            .as_ref()
            .map(|s| format!("LH {} / DN {}", s.last_hits, s.denies))
            .unwrap_or_default(),

        "abilities_header" => "-- ABILITIES --".to_string(),
        l if l.starts_with("ability_") => {
            if let Ok(idx) = l[8..].parse::<usize>() {
                gs.abilities
                    .get(idx)
                    .map(|a| {
                        let name = ability_display_name(&a.name);
                        if a.cooldown > 0.0 {
                            format!("[{}] {} CD:{:.0}s", a.level, name, a.cooldown)
                        } else {
                            format!("[{}] {}", a.level, name)
                        }
                    })
                    .unwrap_or_else(|| "---".to_string())
            } else {
                String::new()
            }
        }

        "game_header" => "-- GAME --".to_string(),
        "gametime" => {
            if let Some(t) = gs.game_time {
                let secs = t as i64;
                let m = secs / 60;
                let s = (secs % 60).abs();
                format!("Time: {}:{:02}", m, s)
            } else {
                "Time: --:--".to_string()
            }
        }
        "daytime" => match gs.daytime {
            Some(true) => "Day".to_string(),
            Some(false) => "Night".to_string(),
            None => String::new(),
        },
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

    let is_spectating = payload.get("player").and_then(|p| p.get("team2")).is_some();

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

    let radiant_score = payload
        .get("map")
        .and_then(|m| m.get("radiant_score"))
        .and_then(|v| v.as_u64())
        .map(|v| v as u32);

    let dire_score = payload
        .get("map")
        .and_then(|m| m.get("dire_score"))
        .and_then(|v| v.as_u64())
        .map(|v| v as u32);

    let buyback_cost = if !is_spectating {
        payload
            .get("player")
            .and_then(|p| p.get("buyback_cost"))
            .and_then(|v| v.as_u64())
            .map(|v| v as u32)
    } else {
        None
    };

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
        radiant_score,
        dire_score,
        buyback_cost,
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
        max_health: data.get("max_health").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        mana: data.get("mana").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        max_mana: data.get("max_mana").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
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
                let name = slot
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("empty")
                    .to_string();
                let charges = slot.get("charges").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                items.push(ItemSlot { name, charges });
            } else {
                items.push(ItemSlot {
                    name: "empty".to_string(),
                    charges: 0,
                });
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
                let name = ab
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("")
                    .to_string();
                if name.is_empty() || name.starts_with("generic_hidden") {
                    continue;
                }
                abilities.push(AbilitySlot {
                    name,
                    level: ab.get("level").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                    cooldown: ab.get("cooldown").and_then(|v| v.as_f64()).unwrap_or(0.0),
                    can_cast: ab
                        .get("can_cast")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                    ultimate: ab
                        .get("ultimate")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
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

async fn ensure_hero_image_cached(
    state: &SharedDota2State,
    short_name: &str,
) -> Result<(), String> {
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
        return Err(format!("Hero image download failed: {}", response.status()));
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
    image::imageops::overlay(
        &mut canvas,
        &resized.to_rgba8(),
        offset_x as i64,
        offset_y as i64,
    );
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

async fn handle_items_composite(State(state): State<SharedDota2State>) -> Response {
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
    let vdf_path = default_steam.join("steamapps").join("libraryfolders.vdf");

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
