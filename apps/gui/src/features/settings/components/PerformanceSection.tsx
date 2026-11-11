import { toast } from '@/lib/toast'
import type { ProcessLoad } from '@/api/daemon'
import { usePerfSample, type PerfSample } from '@/lib/perf-meter'
import type { SettingsPreferences } from '../preferences'
import { buildPerformanceReport, formatCpu, formatMemory, formatRate, sumCpu, sumMemory } from '../performance-report'
import { environmentOf, useProcessSample } from '../performance-sample'
import { Card, Row } from './DesktopSettings'
import { Switch } from '@/components/ui/switch'
import { copyText } from '@/lib/clipboard'

type Update = <K extends keyof SettingsPreferences>(key: K, value: SettingsPreferences[K]) => void

type PerformanceSectionProps = {
  preferences: SettingsPreferences
  update: Update
  appVersion: string | null
}

const PENDING = '…'

function Value({ children }: { children: string }) {
  return <span className="mx-scard-value">{children}</span>
}

function framesText(sample: PerfSample | null): string {
  if (sample === null) return PENDING
  if (!sample.visible) return 'window hidden'
  return `${String(sample.framesPerSecond)} frames/s · ${String(sample.slowFrames)} slow · worst ${String(sample.worstFrameMs)} ms`
}

function loadText(loads: readonly ProcessLoad[]): string {
  if (loads.length === 0) return 'none'
  return `${formatCpu(sumCpu(loads))} · ${formatMemory(sumMemory(loads))}`
}

const WEBVIEW_ICON = <path d="M3 5h18v12H3zM8 21h8M12 17v4" />
const PROCESS_ICON = <path d="M4 4h16v16H4zM9 9h6v6H9M12 1v3M12 20v3M1 12h3M20 12h3" />
const OVERLAY_ICON = <path d="M3 17l5-6 4 4 5-8 4 5" />

export function PerformanceSection({ preferences, update, appVersion }: PerformanceSectionProps) {
  const sample = usePerfSample()
  const processes = useProcessSample()
  const loads = processes.data ?? null
  const copyReport = () => {
    const report = buildPerformanceReport(environmentOf(preferences, appVersion), sample, loads)
    copyText(report).then(
      () => toast.success('Copied the performance report', { id: 'performance-report', duration: 1500 }),
      () => toast.error('Could not copy the report', { id: 'performance-report' }),
    )
  }
  const gpu = document.documentElement.dataset.gpu === 'webgl' ? 'WebGL 2' : 'software'
  return (
    <>
      <Card title="Web view" color="var(--ansi-blue)" iconPath={WEBVIEW_ICON}>
        <Row label="Frames" detail="A frame over 50 ms is a hitch you can feel. Idle, this stays near the display's rate.">
          <Value>{framesText(sample)}</Value>
        </Row>
        <Row label="Drawing" detail="A pane drawn by WebGL costs little; one drawn by the DOM rebuilds every changed row.">
          <Value>{sample === null ? PENDING : `${gpu} · ${String(sample.renderers.webgl)} WebGL, ${String(sample.renderers.dom)} DOM`}</Value>
        </Row>
        <Row label="Draw terminals without the GPU" detail="Use this if typed letters show late or the cursor covers the last one. It applies to terminals opened after you change it: restart the app, or reopen the pane.">
          <Switch aria-label="Draw terminals without the GPU" checked={preferences.terminalPlainDrawing} onCheckedChange={(value) => { update('terminalPlainDrawing', value) }} />
        </Row>
        <Row label="Animations running" detail="Each one repaints its card every frame, even while nothing else happens.">
          <Value>{sample === null ? PENDING : `${String(sample.animations)} · ${String(sample.domNodes)} elements`}</Value>
        </Row>
        <Row label="Terminal output" detail="What the panes receive, and how long xterm takes to parse each write.">
          <Value>{sample === null ? PENDING : `${formatRate(sample.outputBytesPerSecond)} · ${String(sample.flushesPerSecond)} writes/s · ${String(sample.writeLatencyMs)} ms`}</Value>
        </Row>
      </Card>
      <Card
        title="Processes"
        color="var(--ansi-green)"
        iconPath={PROCESS_ICON}
        meta={<span className="mx-scard-value">CPU is a share of one core{loads ? ` of ${String(loads.cores)}` : ''}</span>}
        footRow
        foot={<><button type="button" className="mx-text-button" onClick={copyReport}>Copy report</button><span>Paste it into a bug report, or hand it to an agent.</span></>}
      >
        <Row label="Window" detail="The native host that talks to the daemon.">
          <Value>{loads ? loadText([loads.host]) : PENDING}</Value>
        </Row>
        <Row label="Web view" detail="Draws everything you see. This is where lag usually lives.">
          <Value>{loads ? (loads.webview.length === 0 ? 'not measurable here' : loadText(loads.webview)) : PENDING}</Value>
        </Row>
        <Row label="Daemons" detail="Every maittyd on this machine. More than one usually means a stale one is still running.">
          <Value>{loads ? `${String(loads.daemons.length)} · ${loadText(loads.daemons)}` : PENDING}</Value>
        </Row>
        {loads?.daemons.map((daemon) => (
          <Row key={daemon.pid} label={`${daemon.name} · pid ${String(daemon.pid)}`} detail={daemon.command}>
            <Value>{loadText([daemon])}</Value>
          </Row>
        ))}
        {processes.isError && <Row label="Process figures are unavailable" detail={processes.error.message}><Value>{''}</Value></Row>}
      </Card>
      <Card title="Overlay" color="var(--ansi-mag)" iconPath={OVERLAY_ICON}>
        <Row label="Show the readout over the window" detail="Frames and process load in a corner while you work, so a lag spike can be matched with what caused it.">
          <Switch aria-label="Show the performance overlay" checked={preferences.perfOverlay} onCheckedChange={(value) => { update('perfOverlay', value) }} />
        </Row>
      </Card>
    </>
  )
}
