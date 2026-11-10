let webgl2Available: boolean | null = null

/**
 * Whether the web view can draw with WebGL 2. Asked once per page: it is a property of the web
 * view, not of a pane. Without it the terminal falls back to xterm's DOM renderer, and the window
 * drops the glass finish's shadows, gradients and blur, which a software rasteriser repaints at
 * full cost on every frame (docs/PERFORMANCE.md).
 */
export function hasWebgl2(): boolean {
  webgl2Available ??= document.createElement('canvas').getContext('webgl2') !== null
  return webgl2Available
}

export type GpuKind = 'webgl' | 'software'

export function gpuKind(): GpuKind {
  return hasWebgl2() ? 'webgl' : 'software'
}

/** Marks the document for the stylesheet: `data-gpu` is `webgl` or `software`. */
export function markGpu(): void {
  document.documentElement.dataset.gpu = gpuKind()
}
