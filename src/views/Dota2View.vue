<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { useRoute } from 'vue-router';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import {
  VideoPlay,
  VideoPause,
  Search,
  FolderOpened,
  Check,
  Refresh,
} from '@element-plus/icons-vue';

import {
  saveDota2Settings,
  loadDota2Settings,
  startDota2Integration,
  stopDota2Integration,
  fetchDota2Status,
  autoDetectDota2Path,
  configureDota2Gsi,
} from '../composables/useDota2';
import type { Dota2Status, Dota2HeroInfo, Dota2PlayerStats, Dota2ItemSlot, Dota2AbilitySlot } from '../types/dota2';

const { t } = useI18n();
const route = useRoute();

const deviceIp = computed(() => {
  const decodedId = decodeURIComponent(route.params.id as string);
  if (/^(\d{1,3}\.){3}\d{1,3}$/.test(decodedId)) {
    return decodedId;
  }
  return '';
});

const deviceId = computed(() => route.params.id as string);

const serverRunning = ref(false);
const gameActive = ref(false);
const heroes = ref<Dota2HeroInfo[]>([]);
const gameTime = ref<number | null>(null);
const mapState = ref<string | null>(null);
const daytime = ref<boolean | null>(null);
const playerStats = ref<Dota2PlayerStats | null>(null);
const items = ref<Dota2ItemSlot[]>([]);
const abilities = ref<Dota2AbilitySlot[]>([]);

const port = ref(44444);
const dotaPath = ref<string | null>(null);
const gsiConfigured = ref(false);
const isStarting = ref(false);
const isStopping = ref(false);
const isDetecting = ref(false);
const isConfiguringGsi = ref(false);

let pollTimer: number | undefined;

const formattedGameTime = computed(() => {
  if (gameTime.value === null) return '--:--';
  const totalSecs = Math.floor(gameTime.value);
  const minutes = Math.floor(totalSecs / 60);
  const seconds = totalSecs % 60;
  return `${minutes}:${seconds.toString().padStart(2, '0')}`;
});

const mapStateDisplay = computed(() => {
  if (!mapState.value) return t('dota2.stateUnknown');
  const stateMap: Record<string, string> = {
    DOTA_GAMERULES_STATE_INIT: t('dota2.stateInit'),
    DOTA_GAMERULES_STATE_WAIT_FOR_PLAYERS_TO_LOAD: t('dota2.stateLoading'),
    DOTA_GAMERULES_STATE_HERO_SELECTION: t('dota2.stateHeroPick'),
    DOTA_GAMERULES_STATE_STRATEGY_TIME: t('dota2.stateStrategy'),
    DOTA_GAMERULES_STATE_PRE_GAME: t('dota2.statePreGame'),
    DOTA_GAMERULES_STATE_GAME_IN_PROGRESS: t('dota2.stateInProgress'),
    DOTA_GAMERULES_STATE_POST_GAME: t('dota2.statePostGame'),
  };
  return stateMap[mapState.value] ?? mapState.value;
});

function heroHpPercent(hero: Dota2HeroInfo): number {
  if (hero.max_health === 0) return 0;
  return Math.round((hero.health / hero.max_health) * 100);
}

function heroMpPercent(hero: Dota2HeroInfo): number {
  if (hero.max_mana === 0) return 0;
  return Math.round((hero.mana / hero.max_mana) * 100);
}

function itemDisplayName(raw: string): string {
  const short = raw.startsWith('item_') ? raw.slice(5) : raw;
  if (short === 'empty') return '---';
  return short.split('_').map(w => w.charAt(0).toUpperCase() + w.slice(1)).join(' ');
}

function abilityDisplayName(raw: string): string {
  const parts = raw.split('_');
  const meaningful = parts.length > 1 ? parts.slice(1).join('_') : raw;
  return meaningful.split('_').map(w => w.charAt(0).toUpperCase() + w.slice(1)).join(' ');
}

async function pollStatus() {
  try {
    const status: Dota2Status = await fetchDota2Status();
    serverRunning.value = status.server_running;
    gameActive.value = status.game_state.game_active;
    heroes.value = status.game_state.heroes;
    gameTime.value = status.game_state.game_time ?? null;
    mapState.value = status.game_state.map_state ?? null;
    daytime.value = status.game_state.daytime ?? null;
    playerStats.value = status.game_state.player_stats ?? null;
    items.value = status.game_state.items ?? [];
    abilities.value = status.game_state.abilities ?? [];
  } catch (err) {
    console.error('[Dota2] Poll error:', err);
  }
}

async function handleStart() {
  if (!deviceIp.value) return;
  isStarting.value = true;
  try {
    await startDota2Integration(deviceIp.value, deviceId.value, port.value);
    serverRunning.value = true;
    saveDota2Settings(deviceIp.value, {
      enabled: true,
      port: port.value,
      dota_path: dotaPath.value,
      gsi_configured: gsiConfigured.value,
    });
    ElMessage.success(t('dota2.serverStarted'));
  } catch (err) {
    ElMessage.error(t('dota2.serverStartError', { error: String(err) }));
  } finally {
    isStarting.value = false;
  }
}

async function handleStop() {
  if (!deviceIp.value) return;
  isStopping.value = true;
  try {
    await stopDota2Integration(deviceIp.value);
    serverRunning.value = false;
    gameActive.value = false;
    heroes.value = [];
    saveDota2Settings(deviceIp.value, {
      enabled: false,
      port: port.value,
      dota_path: dotaPath.value,
      gsi_configured: gsiConfigured.value,
    });
    ElMessage.success(t('dota2.serverStopped'));
  } catch (err) {
    ElMessage.error(t('dota2.serverStopError', { error: String(err) }));
  } finally {
    isStopping.value = false;
  }
}

async function handleDetectPath() {
  isDetecting.value = true;
  try {
    const path = await autoDetectDota2Path();
    if (path) {
      dotaPath.value = path;
      ElMessage.success(t('dota2.pathDetected'));
    } else {
      ElMessage.warning(t('dota2.pathNotFound'));
    }
  } catch (err) {
    ElMessage.error(t('dota2.pathDetectError', { error: String(err) }));
  } finally {
    isDetecting.value = false;
  }
}

async function handleSetupGsi() {
  if (!dotaPath.value) return;
  isConfiguringGsi.value = true;
  try {
    const configPath = await configureDota2Gsi(dotaPath.value, port.value);
    gsiConfigured.value = true;
    if (deviceIp.value) {
      saveDota2Settings(deviceIp.value, {
        enabled: serverRunning.value,
        port: port.value,
        dota_path: dotaPath.value,
        gsi_configured: true,
      });
    }
    ElMessage.success(t('dota2.gsiConfigured', { path: configPath }));
  } catch (err) {
    ElMessage.error(t('dota2.gsiConfigError', { error: String(err) }));
  } finally {
    isConfiguringGsi.value = false;
  }
}

onMounted(async () => {
  if (deviceIp.value) {
    const saved = loadDota2Settings(deviceIp.value);
    if (saved) {
      port.value = saved.port || 44444;
      dotaPath.value = saved.dota_path;
      gsiConfigured.value = saved.gsi_configured;
    }
  }

  await pollStatus();
  pollTimer = window.setInterval(pollStatus, 2000);
});

onUnmounted(() => {
  if (pollTimer) {
    window.clearInterval(pollTimer);
  }
});
</script>

<template>
  <div class="dota2-page">
    <div class="dota2-header">
      <div>
        <h2>{{ t('dota2.title') }}</h2>
        <p class="subtitle">{{ t('dota2.subtitle') }}</p>
      </div>
      <div class="header-actions">
        <el-tag :type="serverRunning ? 'success' : 'info'" size="large">
          {{ serverRunning ? t('dota2.serverOn') : t('dota2.serverOff') }}
        </el-tag>
      </div>
    </div>

    <!-- GSI Setup -->
    <el-card class="section-card">
      <template #header>
        <span>{{ t('dota2.gsiSetup') }}</span>
      </template>

      <div class="setup-controls">
        <div class="setup-row">
          <label class="setup-label">{{ t('dota2.dotaPath') }}</label>
          <div class="setup-input-group">
            <el-input v-model="dotaPath" :placeholder="t('dota2.dotaPathPlaceholder')" :disabled="serverRunning"
              style="flex: 1" />
            <el-button :icon="Search" :loading="isDetecting" :disabled="serverRunning" @click="handleDetectPath">
              {{ t('dota2.autoDetect') }}
            </el-button>
          </div>
        </div>

        <div class="setup-row">
          <label class="setup-label">{{ t('dota2.port') }}</label>
          <el-input-number v-model="port" :min="1024" :max="65535" :disabled="serverRunning" style="width: 200px" />
        </div>

        <div class="setup-row">
          <label class="setup-label">{{ t('dota2.gsiConfig') }}</label>
          <div class="setup-actions">
            <el-button type="primary" :icon="FolderOpened" :loading="isConfiguringGsi"
              :disabled="!dotaPath || serverRunning" @click="handleSetupGsi">
              {{ t('dota2.createGsiConfig') }}
            </el-button>
            <el-tag v-if="gsiConfigured" type="success" size="small">
              <el-icon>
                <Check />
              </el-icon>
              {{ t('dota2.gsiReady') }}
            </el-tag>
          </div>
        </div>

        <el-alert v-if="!gsiConfigured" type="info" :title="t('dota2.gsiHintTitle')"
          :description="t('dota2.gsiHintDesc')" show-icon :closable="false" style="margin-top: 8px" />
      </div>
    </el-card>

    <!-- Integration Control -->
    <el-card class="section-card">
      <template #header>
        <div class="section-header-row">
          <span>{{ t('dota2.integration') }}</span>
          <el-button :icon="Refresh" size="small" circle @click="pollStatus" />
        </div>
      </template>

      <div class="setup-controls">
        <div class="setup-row">
          <label class="setup-label">{{ t('dota2.gsiServer') }}</label>
          <div class="setup-actions">
            <el-button v-if="!serverRunning" type="success" :icon="VideoPlay" :loading="isStarting"
              :disabled="!deviceIp" @click="handleStart">
              {{ t('dota2.start') }}
            </el-button>
            <el-button v-else type="danger" :icon="VideoPause" :loading="isStopping" @click="handleStop">
              {{ t('dota2.stop') }}
            </el-button>
          </div>
        </div>

        <div v-if="serverRunning" class="status-info">
          <el-icon color="var(--el-color-success)">
            <VideoPlay />
          </el-icon>
          <span>{{ t('dota2.listeningOn', { port }) }}</span>
        </div>
      </div>
    </el-card>

    <!-- Live Game State -->
    <el-card v-if="serverRunning" class="section-card">
      <template #header>
        <span>{{ t('dota2.gameState') }}</span>
      </template>

      <div v-if="!gameActive" class="empty-state">
        {{ t('dota2.waitingForGame') }}
      </div>

      <div v-else class="game-info">
        <div class="game-meta">
          <el-tag type="success">{{ mapStateDisplay }}</el-tag>
          <span class="game-timer">{{ formattedGameTime }}</span>
          <el-tag v-if="daytime !== null" :type="daytime ? 'warning' : 'info'">
            {{ daytime ? t('dota2.day') : t('dota2.night') }}
          </el-tag>
        </div>

        <div class="heroes-grid">
          <div v-for="(hero, index) in heroes" :key="index" class="hero-card" :class="{ dead: !hero.alive }">
            <div class="hero-header">
              <span class="hero-name">{{ hero.display_name }}</span>
              <el-tag size="small" type="info">Lv {{ hero.level }}</el-tag>
            </div>
            <span v-if="hero.player_name" class="hero-player-name">{{ hero.player_name }}</span>

            <div class="hero-bars">
              <div class="bar-row">
                <span class="bar-label">HP</span>
                <el-progress :percentage="heroHpPercent(hero)" :stroke-width="14" color="#4CAF50"
                  :format="() => `${hero.health}/${hero.max_health}`" />
              </div>
              <div class="bar-row">
                <span class="bar-label">MP</span>
                <el-progress :percentage="heroMpPercent(hero)" :stroke-width="14" color="#2196F3"
                  :format="() => `${hero.mana}/${hero.max_mana}`" />
              </div>
            </div>

            <div v-if="!hero.alive" class="dead-overlay">
              {{ t('dota2.dead') }}
            </div>

            <div class="hero-screen-label">
              {{ t('dota2.screenN', { n: index + 1 }) }}
            </div>
          </div>
        </div>

        <div v-if="heroes.length === 1" class="solo-extras">
          <div v-if="playerStats" class="extra-panel">
            <h4>{{ t('dota2.statsTitle') }}</h4>
            <div class="stats-grid">
              <div class="stat-item">
                <span class="stat-value kda">{{ playerStats.kills }} / {{ playerStats.deaths }} / {{ playerStats.assists
                  }}</span>
                <span class="stat-label">KDA</span>
              </div>
              <div class="stat-item">
                <span class="stat-value gold">{{ playerStats.gold }}</span>
                <span class="stat-label">{{ t('dota2.gold') }}</span>
              </div>
              <div class="stat-item">
                <span class="stat-value">{{ playerStats.gpm }}</span>
                <span class="stat-label">GPM</span>
              </div>
              <div class="stat-item">
                <span class="stat-value">{{ playerStats.xpm }}</span>
                <span class="stat-label">XPM</span>
              </div>
              <div class="stat-item">
                <span class="stat-value">{{ playerStats.last_hits }} / {{ playerStats.denies }}</span>
                <span class="stat-label">LH / DN</span>
              </div>
            </div>
          </div>

          <div v-if="items.length > 0" class="extra-panel">
            <h4>{{ t('dota2.itemsTitle') }}</h4>
            <div class="items-grid">
              <div v-for="(item, idx) in items" :key="idx" class="item-slot">
                {{ itemDisplayName(item.name) }}
                <span v-if="item.charges > 0" class="item-charges">({{ item.charges }})</span>
              </div>
            </div>
          </div>

          <div v-if="abilities.length > 0" class="extra-panel">
            <h4>{{ t('dota2.abilitiesTitle') }}</h4>
            <div class="abilities-grid">
              <div v-for="(ab, idx) in abilities" :key="idx" class="ability-slot"
                :class="{ 'on-cooldown': ab.cooldown > 0, 'ultimate': ab.ultimate }">
                <span class="ability-name">{{ abilityDisplayName(ab.name) }}</span>
                <el-tag size="small" type="info">Lv {{ ab.level }}</el-tag>
                <span v-if="ab.cooldown > 0" class="ability-cd">CD: {{ Math.ceil(ab.cooldown) }}s</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </el-card>
  </div>
</template>

<style scoped>
.dota2-page {
  max-width: 1200px;
  margin: 0 auto;
  padding: 20px;
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.dota2-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 20px;
  flex-wrap: wrap;
}

.dota2-header h2 {
  margin: 0;
  font-size: 28px;
  color: var(--el-text-color-primary);
}

.subtitle {
  margin: 4px 0 0;
  color: var(--el-text-color-secondary);
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 16px;
}

.section-card {
  display: flex;
  flex-direction: column;
}

.section-header-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.setup-controls {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.setup-row {
  display: flex;
  align-items: center;
  gap: 16px;
}

.setup-label {
  min-width: 180px;
  font-size: 14px;
  color: var(--el-text-color-primary);
  font-weight: 500;
}

.setup-input-group {
  display: flex;
  gap: 10px;
  flex: 1;
}

.setup-actions {
  display: flex;
  align-items: center;
  gap: 10px;
}

.status-info {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 14px;
  background-color: var(--el-fill-color-light);
  border-radius: 6px;
  font-size: 13px;
  color: var(--el-text-color-secondary);
}

.empty-state {
  text-align: center;
  padding: 40px 20px;
  color: var(--el-text-color-secondary);
  font-size: 15px;
}

.game-info {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.game-meta {
  display: flex;
  align-items: center;
  gap: 16px;
}

.game-timer {
  font-size: 20px;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
  color: var(--el-text-color-primary);
}

.heroes-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 16px;
}

.hero-card {
  position: relative;
  padding: 14px;
  border-radius: 8px;
  background-color: var(--el-fill-color-lighter);
  border: 1px solid var(--el-border-color-lighter);
  display: flex;
  flex-direction: column;
  gap: 10px;
  transition: opacity 0.2s;
}

.hero-card.dead {
  opacity: 0.5;
}

.hero-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.hero-name {
  font-weight: 600;
  font-size: 14px;
  color: var(--el-text-color-primary);
}

.hero-player-name {
  font-size: 12px;
  color: var(--el-text-color-secondary);
  font-style: italic;
}

.hero-bars {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.bar-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.bar-label {
  min-width: 24px;
  font-size: 12px;
  font-weight: 600;
  color: var(--el-text-color-secondary);
}

.dead-overlay {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  font-size: 18px;
  font-weight: 700;
  color: var(--el-color-danger);
  text-transform: uppercase;
  letter-spacing: 2px;
}

.hero-screen-label {
  font-size: 11px;
  color: var(--el-text-color-placeholder);
  text-align: right;
}

.solo-extras {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: 16px;
}

.extra-panel {
  padding: 14px;
  border-radius: 8px;
  background-color: var(--el-fill-color-lighter);
  border: 1px solid var(--el-border-color-lighter);
}

.extra-panel h4 {
  margin: 0 0 12px 0;
  color: var(--el-text-color-primary);
  font-size: 14px;
}

.stats-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(100px, 1fr));
  gap: 12px;
}

.stat-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
}

.stat-value {
  font-size: 18px;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
  color: var(--el-text-color-primary);
}

.stat-value.kda {
  color: var(--el-color-danger);
}

.stat-value.gold {
  color: #FFD700;
}

.stat-label {
  font-size: 11px;
  color: var(--el-text-color-secondary);
  text-transform: uppercase;
}

.items-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 8px;
}

.item-slot {
  padding: 8px;
  border-radius: 6px;
  background-color: var(--el-fill-color);
  font-size: 13px;
  text-align: center;
  color: var(--el-text-color-primary);
}

.item-charges {
  color: var(--el-text-color-secondary);
  font-size: 11px;
}

.abilities-grid {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.ability-slot {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 10px;
  border-radius: 6px;
  background-color: var(--el-fill-color);
}

.ability-slot.on-cooldown {
  opacity: 0.6;
}

.ability-slot.ultimate {
  border-left: 3px solid var(--el-color-warning);
}

.ability-name {
  flex: 1;
  font-size: 13px;
  color: var(--el-text-color-primary);
}

.ability-cd {
  font-size: 12px;
  color: var(--el-color-danger);
  font-weight: 600;
}

@media (max-width: 768px) {
  .setup-row {
    flex-direction: column;
    align-items: flex-start;
    gap: 8px;
  }

  .setup-label {
    min-width: auto;
  }

  .setup-input-group {
    width: 100%;
    flex-direction: column;
  }

  .heroes-grid {
    grid-template-columns: 1fr;
  }
}
</style>
