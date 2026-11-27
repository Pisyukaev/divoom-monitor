import { invoke } from '@tauri-apps/api/core';
import { readFile } from '@tauri-apps/plugin-fs';

import {
  DEFAULT_DEVICE_FONT,
  DEFAULT_TEXT_ALIGNMENT,
  DEFAULT_TEXT_FONT,
  DEFAULT_TEXT_SIZE,
  DEVICE_FONTS,
  FONT_OPTIONS,
  LIVE_TEXT_HEIGHT,
  LIVE_VALUES,
  TEXT_ALIGNMENT_OPTIONS,
} from '../constants';
import type { ScreenConfig, ScreenImage, TextElement } from '../types/screen';

/**
 * A Times Gate screen is composed here (image + texts on a 128 × 128 canvas)
 * and sent as one picture with `upload_image_data` → Draw/SendHttpGif. The
 * editor preview shows the same rendered picture.
 *
 * Live texts (with `{cpu}`-style placeholders) are the exception: re-sending
 * the picture on every change makes the screen flash "loading...", so the
 * picture without them becomes a static background and the device draws and
 * updates the values itself (`send_live_screen`, src-tauri/src/live_screen.rs).
 * The preview draws them with sample values in an approximate font.
 */
export const SCREEN_SIZE = 128;

const PLACEHOLDER = /\{([a-z_]+)\}/g;
const LIVE_KEYS = new Set(LIVE_VALUES.map((v) => v.key));

/** True if the text has at least one known placeholder such as `{cpu}`. */
export function isLiveText(text: Pick<TextElement, 'content'>): boolean {
  return [...text.content.matchAll(PLACEHOLDER)].some((m) => LIVE_KEYS.has(m[1]));
}

/** The text as the preview shows it: placeholders replaced by sample values. */
export function fillSamples(content: string): string {
  return content.replace(PLACEHOLDER, (match, key: string) =>
    LIVE_VALUES.find((v) => v.key === key)?.sample ?? match);
}

export function deviceFont(font: number | undefined): number {
  return DEVICE_FONTS.some((f) => f.value === font) ? (font as number) : DEFAULT_DEVICE_FONT;
}

export function textFont(font: number | undefined): number {
  return FONT_OPTIONS.some((f) => f.value === font) ? (font as number) : DEFAULT_TEXT_FONT;
}

/** Older configs used 0 (scroll) and 4 (left); both are drawn as left. */
export function textAlignment(alignment: number | undefined): number {
  return TEXT_ALIGNMENT_OPTIONS.some((a) => a.value === alignment)
    ? (alignment as number)
    : DEFAULT_TEXT_ALIGNMENT;
}

export function textSize(text: Pick<TextElement, 'fontSize'>): number {
  return text.fontSize ?? DEFAULT_TEXT_SIZE;
}

export function textFontFamily(font: number | undefined): string {
  return FONT_OPTIONS.find((f) => f.value === textFont(font))?.family ?? 'sans-serif';
}

/** Height of a text's box: the font size plus room for descenders (a device line for live texts). */
export function textBoxHeight(text: Pick<TextElement, 'fontSize' | 'content'>): number {
  if (isLiveText(text)) return LIVE_TEXT_HEIGHT;
  return Math.ceil(textSize(text) * 1.25);
}

const imageCache = new Map<string, Promise<HTMLImageElement>>();

async function imageSource(image: ScreenImage): Promise<string> {
  if (image.type === 'local') {
    const bytes = await readFile(image.source);
    return URL.createObjectURL(new Blob([bytes]));
  }
  // Fetched by Rust: a cross-origin <img> would taint the canvas and block export.
  return invoke<string>('fetch_image_base64', { url: image.source });
}

function decodeImage(src: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => resolve(img);
    img.onerror = () => reject(new Error('Failed to decode image'));
    img.src = src;
  });
}

/** Loads (once per source) the image of a screen. Failed loads are retried next time. */
export function loadScreenImage(image: ScreenImage): Promise<HTMLImageElement> {
  const key = `${image.type}:${image.source}`;
  let promise = imageCache.get(key);
  if (!promise) {
    promise = imageSource(image).then(decodeImage);
    promise.catch(() => imageCache.delete(key));
    imageCache.set(key, promise);
  }
  return promise;
}

/** Draws like CSS `object-fit: cover`: fills the screen, crops the overflow. */
function drawCover(ctx: CanvasRenderingContext2D, img: HTMLImageElement) {
  const scale = Math.max(SCREEN_SIZE / img.naturalWidth, SCREEN_SIZE / img.naturalHeight);
  const w = img.naturalWidth * scale;
  const h = img.naturalHeight * scale;
  ctx.imageSmoothingQuality = 'high';
  ctx.drawImage(img, (SCREEN_SIZE - w) / 2, (SCREEN_SIZE - h) / 2, w, h);
}

function drawText(ctx: CanvasRenderingContext2D, text: TextElement) {
  if (!text.content) return;
  const live = isLiveText(text);
  let content = text.content;
  ctx.save();
  ctx.beginPath();
  ctx.rect(text.x, text.y, text.textWidth, textBoxHeight(text));
  ctx.clip(); // text wider than its area is cut off
  if (live) {
    const font = DEVICE_FONTS.find((f) => f.value === deviceFont(text.deviceFont));
    ctx.font = `bold ${font?.previewSize ?? 10}px Verdana, sans-serif`;
    content = fillSamples(content);
    if (font?.chars) content = [...content].filter((c) => font.chars!.includes(c)).join('');
  } else {
    ctx.font = `${textSize(text)}px ${textFontFamily(text.font)}`;
  }
  ctx.fillStyle = text.color || '#FFFFFF';
  ctx.textBaseline = 'top';
  const align = textAlignment(text.alignment);
  ctx.textAlign = align === 2 ? 'center' : align === 3 ? 'right' : 'left';
  const x = align === 2 ? text.x + text.textWidth / 2 : align === 3 ? text.x + text.textWidth : text.x;
  ctx.fillText(content, x, text.y);
  ctx.restore();
}

export interface RenderedScreen {
  /** PNG data URL, 128 × 128. */
  dataUrl: string;
  /** Set when the screen's image could not be loaded; the rest is still drawn. */
  imageError: string | null;
}

/**
 * Draws the screen. `liveTexts: false` leaves live texts out (the background
 * sent to the device, which draws them itself); the preview draws them with samples.
 */
export async function renderScreen(
  config: Pick<ScreenConfig, 'image' | 'texts'>,
  { liveTexts = true }: { liveTexts?: boolean } = {},
): Promise<RenderedScreen> {
  const canvas = document.createElement('canvas');
  canvas.width = SCREEN_SIZE;
  canvas.height = SCREEN_SIZE;
  const ctx = canvas.getContext('2d');
  if (!ctx) throw new Error('Canvas 2D is not available');

  ctx.fillStyle = '#000000';
  ctx.fillRect(0, 0, SCREEN_SIZE, SCREEN_SIZE);

  let imageError: string | null = null;
  if (config.image) {
    try {
      drawCover(ctx, await loadScreenImage(config.image));
    } catch (e) {
      imageError = e instanceof Error ? e.message : String(e);
    }
  }

  for (const text of config.texts) {
    if (liveTexts || !isLiveText(text)) drawText(ctx, text);
  }

  return { dataUrl: canvas.toDataURL('image/png'), imageError };
}

/**
 * Renders a screen and sends it to the device: as one picture, or, when it has
 * live texts, as a background plus values the device keeps updating. Throws if
 * the image can't be loaded, so a broken image never silently replaces the
 * screen with black.
 */
export async function sendScreen(ipAddress: string, config: ScreenConfig): Promise<void> {
  const live = config.texts.filter(isLiveText);
  const { dataUrl, imageError } = await renderScreen(config, { liveTexts: live.length === 0 });
  if (imageError) throw new Error(imageError);

  if (live.length === 0) {
    await invoke('upload_image_data', {
      ipAddress,
      screenIndex: config.screenIndex,
      data: dataUrl,
    });
    return;
  }

  // Fields inside a struct argument keep their Rust (snake_case) names.
  await invoke('send_live_screen', {
    ipAddress,
    screenIndex: config.screenIndex,
    background: dataUrl,
    texts: live.map((text) => ({
      id: text.id,
      template: text.content,
      x: text.x,
      y: text.y,
      width: text.textWidth,
      font: deviceFont(text.deviceFont),
      color: (text.color || '#FFFFFF').toUpperCase(),
      align: textAlignment(text.alignment),
    })),
  });
}
