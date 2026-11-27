export const dateFormats = [
  {
    value: 'yyyy-mm-dd',
    label: 'yyyy-mm-dd',
  },
  {
    value: 'dd-mm-yyyy',
    label: 'dd-mm-yyyy',
  },
  {
    value: 'mm-dd-yyyy',
    label: 'mm-dd-yyyy',
  },
  {
    value: 'yyyy.mm.dd',
    label: 'yyyy.mm.dd',
  },
  {
    value: 'dd.mm.yyyy',
    label: 'dd.mm.yyyy',
  },
  {
    value: 'mm.dd.yyyy',
    label: 'mm.dd.yyyy',
  },
];

/**
 * Text alignment inside its area (1 left, 2 center, 3 right). Older configs
 * also have 0 (scroll) and 4 (left); both are drawn as left.
 * `labelKey` is under `textElement`.
 */
export const TEXT_ALIGNMENT_OPTIONS: { labelKey: string; value: number }[] = [
  { labelKey: 'alignLeft', value: 1 },
  { labelKey: 'alignCenter', value: 2 },
  { labelKey: 'alignRight', value: 3 },
];

/**
 * Fonts for text drawn into the screen image (utils/screenRender.ts), so any
 * font installed on the PC works. Values 0–7 keep the ids older configs saved.
 */
export const FONT_OPTIONS: { label: string; value: number; family: string }[] = [
  { label: 'Arial', value: 0, family: 'Arial, sans-serif' },
  { label: 'Times New Roman', value: 1, family: '"Times New Roman", serif' },
  { label: 'Verdana', value: 2, family: 'Verdana, sans-serif' },
  { label: 'Courier New', value: 3, family: '"Courier New", monospace' },
  { label: 'Georgia', value: 4, family: 'Georgia, serif' },
  { label: 'Garamond', value: 5, family: 'Garamond, "Times New Roman", serif' },
  { label: 'Comic Sans MS', value: 6, family: '"Comic Sans MS", sans-serif' },
  { label: 'Impact', value: 7, family: 'Impact, sans-serif' },
  { label: 'Segoe UI', value: 8, family: '"Segoe UI", sans-serif' },
  { label: 'Tahoma', value: 9, family: 'Tahoma, sans-serif' },
  { label: 'Consolas', value: 10, family: 'Consolas, monospace' },
];

export const DEFAULT_TEXT_FONT = 0;
export const DEFAULT_TEXT_ALIGNMENT = 1;
/** Font size in device pixels (the LCD is 128 × 128). */
export const DEFAULT_TEXT_SIZE = 16;
export const MIN_TEXT_SIZE = 6;
export const MAX_TEXT_SIZE = 64;

/**
 * Placeholders for live values, typed into a screen text as `{key}`. Keys must
 * match `placeholder_value` in src-tauri/src/live_screen.rs. `sample` stands in
 * for the value in the editor preview.
 */
export const LIVE_VALUES: { key: string; labelKey: string; sample: string }[] = [
  { key: 'cpu', labelKey: 'cpu', sample: '42' },
  { key: 'cpu_temp', labelKey: 'cpuTemp', sample: '55' },
  { key: 'gpu', labelKey: 'gpu', sample: '37' },
  { key: 'gpu_temp', labelKey: 'gpuTemp', sample: '61' },
  { key: 'ram', labelKey: 'ram', sample: '48' },
  { key: 'ram_used', labelKey: 'ramUsed', sample: '7.6' },
  { key: 'ram_total', labelKey: 'ramTotal', sample: '15.9' },
  { key: 'disk', labelKey: 'disk', sample: '63' },
];

/**
 * Device fonts for live values, checked on a Times Gate. The device draws live
 * values itself, so only its own fonts can be used, and an id it lacks hangs
 * the whole screen on "loading...": don't add ids without trying them.
 * `previewSize` approximates the glyph height for the editor preview.
 * `chars`, when set, are the only characters the font draws; the preview
 * drops the rest like the device does.
 */
export const DEVICE_FONTS: { value: number; noteKey?: string; previewSize: number; chars?: string }[] = [
  { value: 4, previewSize: 10 },
  { value: 2, previewSize: 10 },
  { value: 32, previewSize: 11 },
  { value: 52, previewSize: 10 },
  // On a Times Gate "F20 42% 55C" showed as "2042%55" (the % looks like a
  // slash): letters and spaces are skipped. Other punctuation is untested.
  { value: 20, noteKey: 'deviceFontDigits', previewSize: 14, chars: '0123456789%' },
  { value: 18, noteKey: 'deviceFontTiny', previewSize: 7 },
];
export const DEFAULT_DEVICE_FONT = 4;
/** Height of a live value's line on the device, in pixels. */
export const LIVE_TEXT_HEIGHT = 16;

export const TEXT_IDS: number[] = [
  0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
];

export const commands = [
  'scan_devices',
  'set_brightness',
  'set_switch_screen',
  'set_temperature_mode',
  'set_mirror_mode',
  'set_24_hours_mode',
  'get_device_info',
  'upload_image_from_url',
  'upload_image_from_file',
  'reboot_device',
  'get_lcd_info',
  'activate_pc_monitor',
  'send_pc_metrics',
] as const;
