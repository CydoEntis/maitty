import { isModifierSymbol, type Chord } from '../model/shortcuts'
import { Icon, Spinner } from './glyphs'

type Props = {
  online: boolean
  /** The focused session's git branch, if it is in a repo. */
  branch: string | null
  working: number
  needs: number
  /** The focused session's process and folder. */
  process: string | null
  path: string | null
  /** The terminal text size in px. */
  textSize: number
  shortcutChord: Chord
  onOpenInbox: () => void
  onOpenShortcuts: () => void
}

/** What is going on across every session, and where you are, in one quiet line at the bottom of the window. */
export function StatusBar(props: Props) {
  return (
    <footer className="mx-statusbar" aria-label="Status">
      <span className="mx-sb-item">
        <span className={`mx-daemon-dot${props.online ? '' : ' is-offline'}`} aria-hidden="true" />
        {props.online ? 'maittyd' : 'reconnecting…'}
      </span>
      {props.branch && <span className="mx-sb-item mono"><Icon name="branch" />{props.branch}</span>}
      {props.working > 0 && <span className="mx-sb-item is-working"><Spinner />{props.working} working</span>}
      {props.needs > 0 && <button type="button" className="mx-sb-item is-needs" title="Open the inbox" onClick={props.onOpenInbox}>{props.needs} needs you</button>}
      <span className="mx-spacer" />
      {props.process && <span className="mx-sb-item mono">{props.process}{props.path ? ` · ${props.path}` : ''}</span>}
      <span className="mx-sb-item">{props.textSize}px</span>
      <button type="button" className="mx-sb-item" aria-label="Keyboard shortcuts" title="Keyboard shortcuts" onClick={props.onOpenShortcuts}>
        {props.shortcutChord.map((key) => <kbd key={key} className={`mx-keycap${isModifierSymbol(key) ? ' is-symbol' : ''}`}>{key}</kbd>)}
      </button>
    </footer>
  )
}
