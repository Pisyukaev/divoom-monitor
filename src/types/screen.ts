export interface TextElement {
  id: number;
  content: string;
  x: number;
  y: number;
  /** Index into FONT_OPTIONS. */
  font?: number;
  /** Font size in device pixels; DEFAULT_TEXT_SIZE when missing (older configs). */
  fontSize?: number;
  color?: string;
  alignment?: number;
  textWidth: number;
  /**
   * Font of a live text (one with `{cpu}`-style placeholders), which the device
   * draws itself: an id from DEVICE_FONTS. `font`/`fontSize` apply to static texts only.
   */
  deviceFont?: number;
}

export interface ScreenImage {
  type: 'local' | 'url';
  source: string;
}

export interface ScreenConfig {
  screenIndex: number;
  image?: ScreenImage;
  texts: TextElement[];
  textIds: number[];
}

export interface ScreenConfigs {
  [screenIndex: number]: ScreenConfig;
}
