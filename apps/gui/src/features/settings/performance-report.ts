import type { ProcessLoad, ProcessSample } from '@/api/daemon'
import type { PerfSample } from '@/lib/perf-meter'

/** What the window knows about itself that bears on how fast it draws. */
export type PerformanceEnvironment = {
  version: string | null
  userAgent: string
  /** `webgl` or `software`, as `data-gpu` on the root. */
  gpu: string
  corners: string
  reduceMotion: boolean
  terminalFontSize: number
}

const BYTES_PER_KIB = 1024
const BYTES_PER_MIB = BYTES_PER_KIB * BYTES_PER_KIB

export function formatMemory(bytes: number): string {
  if (bytes >= BYTES_PER_MIB) return `${(bytes / BYTES_PER_MIB).toFixed(0)} MiB`
  return `${(bytes / BYTES_PER_KIB).toFixed(0)} KiB`
}

export function formatRate(bytesPerSecond: number): string {
  if (bytesPerSecond >= BYTES_PER_MIB) return `${(bytesPerSecond / BYTES_PER_MIB).toFixed(1)} MiB/s`
  if (bytesPerSecond >= BYTES_PER_KIB) return `${(bytesPerSecond / BYTES_PER_KIB).toFixed(1)} KiB/s`
  return `${String(bytesPerSecond)} B/s`
}

export function formatCpu(percentOfOneCore: number): string {
  return `${percentOfOneCore.toFixed(0)}%`
}

export function sumCpu(loads: readonly ProcessLoad[]): number {
  return loads.reduce((total, load) => total + load.cpu, 0)
}

export function sumMemory(loads: readonly ProcessLoad[]): number {
  return loads.reduce((total, load) => total + load.memory_bytes, 0)
}

function loadLine(load: ProcessLoad): string {
  const command = load.command === '' ? load.name : load.command
  return `  ${String(load.pid)}  ${formatCpu(load.cpu)} cpu  ${formatMemory(load.memory_bytes)}  ${command}`
}

/**
 * A plain-text report to paste into a bug report or hand to an agent: the environment, the last
 * web view sample and the last process sample. Lines are stable so two reports can be diffed.
 */
export function buildPerformanceReport(environment: PerformanceEnvironment, sample: PerfSample | null, processes: ProcessSample | null): string {
  const lines: string[] = [
    `Maitty desktop ${environment.version ?? '(version unknown)'}`,
    `Web view: ${environment.userAgent}`,
    `GPU: ${environment.gpu}`,
    `Corners: ${environment.corners}; reduce motion ${environment.reduceMotion ? 'on' : 'off'}; terminal ${String(environment.terminalFontSize)}px`,
    '',
  ]
  if (sample === null) {
    lines.push('Web view sample: none yet')
  } else {
    lines.push(
      'Web view (last second)',
      `  frames/s ${String(sample.framesPerSecond)}${sample.visible ? '' : ' (window hidden)'}; slow frames ${String(sample.slowFrames)}; worst frame ${String(sample.worstFrameMs)} ms`,
      `  terminal renderers: ${String(sample.renderers.webgl)} WebGL, ${String(sample.renderers.dom)} DOM`,
      `  output ${formatRate(sample.outputBytesPerSecond)} in ${String(sample.outputFramesPerSecond)} frames/s; ${String(sample.flushesPerSecond)} xterm writes/s, ${String(sample.writeLatencyMs)} ms each`,
      `  output frames received since the window opened: ${String(sample.outputFramesTotal)}, of which over the local socket: ${String(sample.socketFramesTotal)}`,
      sample.keyReplies.count === 0
        ? '  key replies from the host: no keys typed yet'
        : `  key replies from the host (last ${String(sample.keyReplies.count)} keys): median ${String(sample.keyReplies.medianMs)} ms, worst ${String(sample.keyReplies.worstMs)} ms`,
      `  animations running ${String(sample.animations)}; DOM nodes ${String(sample.domNodes)}`,
    )
  }
  lines.push('')
  if (processes === null) {
    lines.push('Processes: none yet')
  } else {
    lines.push(
      `Processes (cpu is percent of one core; ${String(processes.cores)} cores)`,
      `terminal frames the host sent to the window since it started: ${String(processes.terminal_frames_sent)}`,
      'host',
      loadLine(processes.host),
      processes.webview.length === 0 ? 'web view: not a child process here' : 'web view',
      ...processes.webview.map(loadLine),
      processes.daemons.length === 0 ? 'daemons: none found' : 'daemons',
      ...processes.daemons.map(loadLine),
    )
  }
  return lines.join('\n')
}
