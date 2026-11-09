import { useEffect, useRef } from 'react'
import { WebSpinner } from '@/components/WebSpinner'
import { PickerSelect, type PickerGroup } from './PickerSelect'
import { isSpinnerStyle, isWebSpinner, spinnerDefinition, SPINNER_TICK_MS, TUI_SPINNERS, WEB_SPINNERS, type SpinnerStyle, type TuiSpinner } from '@/lib/spinner-clock'

const TUI_SOURCE = 'From the Maitty TUI'
const WEB_SOURCE = 'Web icon'

/** One TUI spinner, stepping on its own timer written straight into the element, so the page never re-renders for a frame. */
function TuiPreview({ spinner, reduced }: { spinner: TuiSpinner; reduced: boolean }) {
  const { frames } = spinnerDefinition(spinner)
  const element = useRef<HTMLSpanElement>(null)
  useEffect(() => {
    const target = element.current
    if (reduced || !target) return
    let frame = 0
    const timer = window.setInterval(() => {
      frame = (frame + 1) % frames.length
      target.textContent = frames[frame] ?? ''
    }, SPINNER_TICK_MS)
    return () => { window.clearInterval(timer) }
  }, [frames, reduced])
  return <span ref={element}>{frames[0]}</span>
}

/** A live preview of any spinner: the TUI's frames, or the web icon moving by CSS. */
export function SpinnerPreview({ spinner, reduced }: { spinner: SpinnerStyle; reduced: boolean }) {
  return isWebSpinner(spinner) ? <WebSpinner kind={spinner} className="" still={reduced} /> : <TuiPreview spinner={spinner} reduced={reduced} />
}

function Tile({ spinner, reduced, size }: { spinner: SpinnerStyle; reduced: boolean; size: string }) {
  return (
    <span aria-hidden="true" className={`grid shrink-0 place-items-center rounded-md bg-term font-mono text-brand ring-1 ring-pane-line ${size}`}>
      <SpinnerPreview spinner={spinner} reduced={reduced} />
    </span>
  )
}

function groupOf(label: string, spinners: ReadonlyArray<readonly [SpinnerStyle, string]>, source: string, reduced: boolean): PickerGroup {
  return { label, items: spinners.map(([id, name]) => ({ value: id, label: name, sub: source, tile: <Tile spinner={id} reduced={reduced} size="size-7 text-[15px]" /> })) }
}

/** The working spinner, picked as the themes and the window style are: the TUI's three and five web icons, each live in its row. */
export function SpinnerPicker({ value, reduced, onPick }: { value: SpinnerStyle; reduced: boolean; onPick: (spinner: SpinnerStyle) => void }) {
  return (
    <PickerSelect
      label="Working spinner"
      value={value}
      groups={[groupOf('Maitty TUI', TUI_SPINNERS, TUI_SOURCE, reduced), groupOf('Web', WEB_SPINNERS, WEB_SOURCE, reduced)]}
      onPick={(next) => { if (isSpinnerStyle(next)) onPick(next) }}
    />
  )
}
