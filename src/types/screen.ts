export interface TextElement {
  id: string;
  content: string;
  x: number;
  y: number;
  fontSize?: number;
  color?: string;
  alignment?: 'left' | 'center' | 'right';
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
