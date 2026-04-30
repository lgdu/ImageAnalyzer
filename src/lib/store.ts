import type { ImageAnalysis } from './types';

class Store {
  fileList: ImageAnalysis[] = $state([]);
  currentImage: ImageAnalysis | null = $state(null);
  isAnalyzing = $state(false);
  error: string | null = $state(null);
}

export const store = new Store();
