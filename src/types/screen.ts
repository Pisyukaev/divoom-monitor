export interface TextElement {
  id: number;
  content: string;
  x: number;
  y: number;
  fontSize?: number;
  color?: string;
  alignment?: 0 | 1 | 2 | 3 | 4;
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
