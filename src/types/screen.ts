export interface TextElement {
  id: number;
  content: string;
  x: number;
  y: number;
  font?: number;
  color?: string;
  alignment?: number;
  textWidth: number;
}

export interface ScreenImage {
  type: 'local' | 'url';
  source: string;
}

export interface ScreenConfig {
  screenIndex: number;
  image?: ScreenImage;
  texts: TextElement[];
}

export interface ScreenConfigs {
  [screenIndex: number]: ScreenConfig;
}
