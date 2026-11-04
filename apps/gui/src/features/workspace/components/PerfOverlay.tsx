import { usePerfSample } from '@/lib/perf-meter'
import { useProcessSample } from '@/features/settings/performance-sample'
import { formatCpu, formatRate, sumCpu } from '@/features/settings/performance-report'

/**
 * A one-line readout in the window's corner (Settings › Performance › Overlay): frames, the web
 * view's CPU and the terminal output rate, once a second, so a lag spike can be matched with what
 * the app was doing at the time.
 */
export function PerfOverlay() {
  const sample = usePerfSample()
  const processes = useProcessSample()
  const loads = processes.data
  const frames = sample === null ? '…' : !sample.visible ? 'hidden' : `${String(sample.framesPerSecond)} fps`
  const slow = sample !== null && sample.slowFrames > 0 ? ` · ${String(sample.slowFrames)} slow` : ''
  const webview = loads === undefined ? '…' : loads.webview.length === 0 ? formatCpu(loads.host.cpu) : formatCpu(sumCpu(loads.webview))
  const output = sample === null ? '…' : formatRate(sample.outputBytesPerSecond)
  const anims = sample === null ? '' : ` · ${String(sample.animations)} anim`
  return (
    <div className="mx-perf-hud" role="status" aria-label="Performance readout">
      <span>{frames}{slow}</span>
      <span>cpu {webview}</span>
      <span>{output}{anims}</span>
    </div>
  )
}
