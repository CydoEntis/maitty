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

/**
 * Whether terminals should start on xterm's DOM renderer. On WebKitGTK (the Linux web view) the WebGL
 * renderer showed typed letters late and left the cursor over the last one while frames, output and
 * key replies were all fast; the DOM renderer fixed it on the machine that reported it (Hyprland,
 * NVIDIA, WebKitGTK 2.52). macOS and Windows keep WebGL. The user can change it in Settings › Performance.
 * WebKitGTK's user agent names X11 and Linux; Android and the test environment's do not.
 */
export function prefersPlainDrawing(userAgent: string = navigator.userAgent): boolean {
  return userAgent.includes('X11; Linux')
}

export type GpuKind = 'webgl' | 'software'

export function gpuKind(): GpuKind {
  return hasWebgl2() ? 'webgl' : 'software'
}

/** Marks the document for the stylesheet: `data-gpu` is `webgl` or `software`. */
export function markGpu(): void {
  document.documentElement.dataset.gpu = gpuKind()
}
