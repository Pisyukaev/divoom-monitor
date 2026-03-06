export interface Dota2HeroInfo {
  name: string;
  display_name: string;
  player_name: string;
  health: number;
  max_health: number;
  mana: number;
  max_mana: number;
  level: number;
  alive: boolean;
}

export interface Dota2PlayerStats {
  kills: number;
  deaths: number;
  assists: number;
  last_hits: number;
  denies: number;
  gold: number;
  gpm: number;
  xpm: number;
}

export interface Dota2ItemSlot {
  name: string;
  charges: number;
}

export interface Dota2AbilitySlot {
  name: string;
  level: number;
  cooldown: number;
  can_cast: boolean;
  ultimate: boolean;
}

export interface Dota2GameState {
  game_active: boolean;
  heroes: Dota2HeroInfo[];
  game_time: number | null;
  map_state: string | null;
  daytime: boolean | null;
  player_stats: Dota2PlayerStats | null;
  items: Dota2ItemSlot[];
  abilities: Dota2AbilitySlot[];
  radiant_score: number | null;
  dire_score: number | null;
  buyback_cost: number | null;
}

export interface Dota2Status {
  server_running: boolean;
  port: number;
  game_state: Dota2GameState;
}

export interface Dota2Settings {
  enabled: boolean;
  port: number;
  dota_path: string | null;
  gsi_configured: boolean;
}
