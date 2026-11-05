import type { ReactNode } from 'react'
import { Copy, Minus, Square, X } from 'lucide-react'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'
import { cn } from '@/lib/utils'
import type { TitleBarPlatform } from './title-bar-platform'

export interface TitleBarActions {
  maximized: boolean
  onMinimize: () => void
  onToggleMaximize: () => void
  onClose: () => void
}

type Kind = 'minimize' | 'maximize' | 'close'

const TOOLTIP_DELAY_MS = 400
const MAC_ICON_SIZE = 8
const ICON_SIZE = 14
const GLYPH_SHRINK = 2

// Windows draws 46x32 captions; Linux 20px round buttons; macOS 12px traffic lights.
const BUTTON_CLASS: Record<TitleBarPlatform, string> = {
  windows:
    'flex h-full w-[46px] items-center justify-center text-(--muted-text) transition-colors hover:bg-(--hover) hover:text-(--fg)',
  linux:
    'my-auto flex size-5 items-center justify-center rounded-full bg-(--hover) text-(--muted-text) transition-colors hover:text-(--fg)',
  macos: 'group/light flex size-3 items-center justify-center rounded-full text-black/60',
}

const CLOSE_CLASS: Record<TitleBarPlatform, string> = {
  windows: 'hover:bg-caption-close hover:text-white',
  linux: 'hover:bg-caption-close hover:text-white',
  macos: '',
}

const LIGHT_COLOR: Record<Kind, string> = {
  close: 'bg-traffic-close',
  minimize: 'bg-traffic-minimize',
  maximize: 'bg-traffic-maximize',
}

function WindowButton(props: { platform: TitleBarPlatform; kind: Kind; label: string; icon: ReactNode; onClick: () => void }) {
  const { platform, kind, label, icon, onClick } = props
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <button
          type="button"
          aria-label={label}
          data-kind={kind}
          className={cn(BUTTON_CLASS[platform], platform === 'macos' && LIGHT_COLOR[kind], kind === 'close' && CLOSE_CLASS[platform])}
          onClick={onClick}
        >
          {/* The traffic lights show their glyph only while hovered, as on macOS. */}
          {platform === 'macos' ? <span className="opacity-0 group-hover/light:opacity-100">{icon}</span> : icon}
        </button>
      </TooltipTrigger>
      <TooltipContent side="bottom">{label}</TooltipContent>
    </Tooltip>
  )
}

function WindowButtons({ platform, actions }: { platform: TitleBarPlatform; actions: TitleBarActions }) {
  const mac = platform === 'macos'
  const iconSize = mac ? MAC_ICON_SIZE : ICON_SIZE
  const smallIconSize = iconSize - GLYPH_SHRINK
  const maximizeLabel = actions.maximized ? 'Restore' : 'Maximize'
  const minimize = (
    <WindowButton key="minimize" platform={platform} kind="minimize" label="Minimize" icon={<Minus size={iconSize} />} onClick={actions.onMinimize} />
  )
  const maximize = (
    <WindowButton
      key="maximize"
      platform={platform}
      kind="maximize"
      label={maximizeLabel}
      icon={actions.maximized ? <Copy size={smallIconSize} /> : <Square size={smallIconSize} />}
      onClick={actions.onToggleMaximize}
    />
  )
  const close = (
    <WindowButton key="close" platform={platform} kind="close" label="Close window" icon={<X size={iconSize + 1} />} onClick={actions.onClose} />
  )
  // macOS reads close, minimize, zoom from the left edge; the others read minimize, maximize, close.
  const buttons = mac ? [close, minimize, maximize] : [minimize, maximize, close]
  return (
    <div
      data-testid="window-controls"
      className={cn('flex h-full items-stretch', mac && 'gap-2 pr-2', platform === 'linux' && 'gap-2 pr-3', !mac && 'ml-auto')}
    >
      {buttons}
    </div>
  )
}

/** The bar itself, free of Tauri so every platform's layout can be rendered in a test. */
export function TitleBarView({ platform, actions }: { platform: TitleBarPlatform; actions: TitleBarActions }) {
  const mac = platform === 'macos'
  return (
    <TooltipProvider delayDuration={TOOLTIP_DELAY_MS}>
      <header className="title-bar" data-tauri-drag-region data-platform={platform}>
        {mac && <WindowButtons platform={platform} actions={actions} />}
        <img className="title-bar-logo" src="/maitty-logo.png" alt="" />
        <span className="title-bar-name">Maitty</span>
        {!mac && <WindowButtons platform={platform} actions={actions} />}
      </header>
    </TooltipProvider>
  )
}
