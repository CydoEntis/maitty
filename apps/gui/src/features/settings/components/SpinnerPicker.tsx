import { useCallback, useEffect, useRef, useState } from 'react'
import { ChevronDownIcon } from 'lucide-react'
import { WebSpinner } from '@/components/WebSpinner'
import { DropdownMenu, DropdownMenuContent, DropdownMenuLabel, DropdownMenuRadioGroup, DropdownMenuRadioItem, DropdownMenuSeparator, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { isSpinnerStyle, isWebSpinner, spinnerDefinition, SPINNER_TICK_MS, TUI_SPINNERS, WEB_SPINNERS, type SpinnerStyle, type TuiSpinner } from '@/lib/spinner-clock'

const TUI_SOURCE = 'From the Maitty TUI'
const WEB_SOURCE = 'Web icon'

function nameOf(spinner: SpinnerStyle): string {
  return [...TUI_SPINNERS, ...WEB_SPINNERS].find(([id]) => id === spinner)?.[1] ?? spinner
}

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

const ITEM_CLASS = 'h-9 gap-2.5 py-0 pr-8 pl-1.5 text-[12.5px] text-fg focus:bg-hover focus:text-fg [&_[data-slot=dropdown-menu-radio-item-indicator]]:text-brand'
const GROUP_LABEL_CLASS = 'px-1.5 pt-1.5 pb-1 text-[10.5px] font-semibold tracking-wider text-dim uppercase'

function Group({ label, spinners, reduced }: { label: string; spinners: ReadonlyArray<readonly [SpinnerStyle, string]>; reduced: boolean }) {
  return (
    <>
      <DropdownMenuLabel className={GROUP_LABEL_CLASS}>{label}</DropdownMenuLabel>
      {spinners.map(([id, name]) => (
        <DropdownMenuRadioItem key={id} value={id} className={ITEM_CLASS}>
          <Tile spinner={id} reduced={reduced} size="size-6 text-[13px]" />
          {name}
        </DropdownMenuRadioItem>
      ))}
    </>
  )
}

/**
 * The working spinner as a dropdown row: a button showing the pick, and a list of the three TUI
 * spinners and five web icons with live previews. The list is Radix's menu (focus, arrows, Esc,
 * outside click and flipping upward when there is no room come from it).
 */
export function SpinnerPicker({ value, reduced, onPick }: { value: SpinnerStyle; reduced: boolean; onPick: (spinner: SpinnerStyle) => void }) {
  // The list mounts inside the app root, where the theme's custom properties are set.
  const [container, setContainer] = useState<HTMLElement | null>(null)
  const findRoot = useCallback((button: HTMLButtonElement | null) => { setContainer(button?.closest<HTMLElement>('.mx-app') ?? null) }, [])
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        {/* 220 x 40 is the size the v18 design gives this button, so it has no scale value. */}
        <button ref={findRoot} type="button" aria-label="Working spinner" className="flex h-10 w-[220px] items-center gap-2.5 rounded-lg bg-raise px-1.5 text-left text-fg ring-1 ring-pane-line outline-none hover:bg-hover focus-visible:ring-2 focus-visible:ring-brand data-[state=open]:ring-brand">
          <Tile spinner={value} reduced={reduced} size="size-7 text-[15px]" />
          <span className="flex min-w-0 flex-1 flex-col">
            <span className="text-[13px] leading-tight font-semibold">{nameOf(value)}</span>
            <span className="text-[11.5px] leading-tight text-dim">{isWebSpinner(value) ? WEB_SOURCE : TUI_SOURCE}</span>
          </span>
          <ChevronDownIcon className="mr-1 size-4 shrink-0 text-dim" aria-hidden="true" />
        </button>
      </DropdownMenuTrigger>
      {/* 260 x 340 is the design's list size. Esc is stopped here so it closes the list and not Settings. */}
      <DropdownMenuContent
        container={container}
        align="start"
        aria-label="Working spinner"
        className="max-h-[min(340px,var(--radix-dropdown-menu-content-available-height))] w-[260px] text-fg"
        onEscapeKeyDown={(event) => { event.stopPropagation() }}
      >
        <DropdownMenuRadioGroup value={value} onValueChange={(next) => { if (isSpinnerStyle(next)) onPick(next) }}>
          <Group label="Maitty TUI" spinners={TUI_SPINNERS} reduced={reduced} />
          <DropdownMenuSeparator className="bg-pane-line" />
          <Group label="Web" spinners={WEB_SPINNERS} reduced={reduced} />
        </DropdownMenuRadioGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
