import { useState, type KeyboardEvent, type ReactNode } from 'react'
import { Icon } from '@/features/workspace/components/glyphs'
import { Collapse, SectionHead } from '@/features/workspace/components/Collapse'
import type { CrewApi } from '../useCrew'
import { CAPTAIN_ID, chatSubtitle, MAX_NAME_LENGTH, whoIs, type CrewAgent, type Mate } from '../model/crew'
import { CrewAvatar } from './CrewAvatar'

type RowProps = { avatar: ReactNode; name: string; tag?: string; sub: string; unread: number; selected: boolean; label: string; tall?: boolean; onPick: () => void }

function CrewRow({ avatar, name, tag, sub, unread, selected, label, tall = false, onPick }: RowProps) {
  return (
    <button type="button" className={`mx-crew-row${selected ? ' is-selected' : ''}${tall ? ' is-tall' : ''}`} aria-label={label} aria-current={selected ? 'true' : undefined} onClick={onPick}>
      {avatar}
      <span className="mx-crew-text">
        <span className="mx-crew-line">
          <span className="mx-crew-name">{name}</span>
          {tag !== undefined && <span className="mx-crew-tag">{tag}</span>}
        </span>
        <span className="mx-crew-sub">{sub}</span>
      </span>
      {unread > 0 && !selected && <span className="mx-crew-unread" aria-label={`${String(unread)} unread`}>{unread}</span>}
    </button>
  )
}

const CAPTAIN_AVATAR = 32
const MATE_AVATAR = 26
const STACK_LIMIT = 4

const AGENTS: ReadonlyArray<{ id: CrewAgent; label: string }> = [{ id: 'claude', label: 'Claude' }, { id: 'codex', label: 'Codex' }]

/** The inline form under CREWMATES: a name, what they do, and Claude or Codex. Enter adds, Esc cancels. */
function NewMateForm({ onAdd, onCancel }: { onAdd: (input: { name: string; role: string; agent: CrewAgent }) => void; onCancel: () => void }) {
  const [name, setName] = useState('')
  const [role, setRole] = useState('')
  const [agent, setAgent] = useState<CrewAgent>('claude')
  const onKeyDown = (event: KeyboardEvent) => {
    if (event.key === 'Escape') { event.preventDefault(); event.stopPropagation(); onCancel() }
    if (event.key === 'Enter' && name.trim() !== '') { event.preventDefault(); onAdd({ name, role, agent }) }
  }
  return (
    <div className="mx-crew-form" role="group" aria-label="New crewmate">
      <input autoFocus value={name} maxLength={MAX_NAME_LENGTH} placeholder="Name your crewmate" aria-label="Crewmate name" spellCheck={false} onChange={(event) => { setName(event.target.value) }} onKeyDown={onKeyDown} />
      <input value={role} placeholder="What do they do? (optional)" aria-label="What the crewmate does" spellCheck={false} onChange={(event) => { setRole(event.target.value) }} onKeyDown={onKeyDown} />
      <div className="mx-crew-form-foot">
        {AGENTS.map((option) => (
          <button key={option.id} type="button" className={`mx-crew-agent${agent === option.id ? ' is-on' : ''}`} aria-pressed={agent === option.id} onClick={() => { setAgent(option.id) }}>
            <span className="mx-crew-agent-mark" aria-hidden="true">✻</span>{option.label}
          </button>
        ))}
        <span className="mx-crew-hint">↵ add · Esc</span>
      </div>
    </div>
  )
}

function NewCrewForm({ onCreate, onCancel }: { onCreate: (name: string) => void; onCancel: () => void }) {
  const [name, setName] = useState('')
  return (
    <div className="mx-crew-form" role="group" aria-label="New crew">
      <input
        autoFocus
        value={name}
        maxLength={MAX_NAME_LENGTH}
        placeholder="Name the crew"
        aria-label="Crew name"
        spellCheck={false}
        onChange={(event) => { setName(event.target.value) }}
        onKeyDown={(event) => {
          if (event.key === 'Escape') { event.preventDefault(); event.stopPropagation(); onCancel() }
          if (event.key === 'Enter' && name.trim() !== '') { event.preventDefault(); onCreate(name) }
        }}
      />
      <span className="mx-crew-hint">Your Captain joins first. Add crewmates or share an invite after. ↵ create · Esc</span>
    </div>
  )
}

/** The sidebar in Crewmates mode: the Captain, the crewmates under it, and the crews. */
export function CrewmatesPanel({ crew }: { crew: CrewApi }) {
  const { state } = crew
  const [folded, setFolded] = useState<ReadonlySet<string>>(new Set())
  const [adding, setAdding] = useState<'mate' | 'crew' | null>(null)
  const sectionOpen = (key: string) => !folded.has(key)
  const toggle = (key: string) => { setFolded((current) => { const next = new Set(current); if (!next.delete(key)) next.add(key); return next }) }
  const reveal = (key: string) => { setFolded((current) => { const next = new Set(current); next.delete(key); return next }) }
  const captain = state.mates.find((mate) => mate.id === CAPTAIN_ID)
  const mates = state.mates.filter((mate) => mate.id !== CAPTAIN_ID)
  const open = (id: string) => { setAdding(null); crew.open(id) }
  const mateRow = (mate: Mate, isCaptain: boolean) => (
    <CrewRow
      key={mate.id}
      avatar={<CrewAvatar who={mate} size={isCaptain ? CAPTAIN_AVATAR : MATE_AVATAR} status={mate.status} />}
      name={mate.name}
      {...(isCaptain ? { tag: 'CAPTAIN' } : {})}
      sub={chatSubtitle(state, mate.id)}
      unread={mate.unread}
      selected={state.selected === mate.id}
      label={`${mate.name}, ${mate.role}`}
      tall={isCaptain}
      onPick={() => { open(mate.id) }}
    />
  )
  return (
    <div className="mx-crew-panel">
      <div className="mx-crew-label">CAPTAIN</div>
      {captain && mateRow(captain, true)}

      <SectionHead label="CREWMATES" count={mates.length} open={sectionOpen('mates')} onToggle={() => { toggle('mates') }} className="is-counted is-crew-head">
        <button type="button" className="mx-icon-button mx-crew-add" aria-label="Add a crewmate" title="Add a crewmate" onClick={() => { reveal('mates'); setAdding('mate') }}><Icon name="plus" size={13} /></button>
      </SectionHead>
      <Collapse open={sectionOpen('mates')}>
        <div className="mx-crew-tree">
          <span className="mx-crew-trunk" aria-hidden="true" />
          {mates.map((mate) => <div key={mate.id} className="mx-crew-branch">{mateRow(mate, false)}</div>)}
          {adding === 'mate' && <NewMateForm onAdd={(input) => { setAdding(null); crew.createMate(input) }} onCancel={() => { setAdding(null) }} />}
        </div>
      </Collapse>

      <SectionHead label="CREWS" count={state.crews.length} open={sectionOpen('crews')} onToggle={() => { toggle('crews') }} className="is-counted is-crew-head">
        <button type="button" className="mx-icon-button mx-crew-add" aria-label="Start a crew" title="Start a crew" onClick={() => { reveal('crews'); setAdding('crew') }}><Icon name="plus" size={13} /></button>
      </SectionHead>
      <Collapse open={sectionOpen('crews')}>
        <div className="mx-crew-list">
          {state.crews.map((entry) => (
            <CrewRow
              key={entry.id}
              avatar={<span className="mx-crew-stack" aria-hidden="true">{entry.members.slice(0, STACK_LIMIT).map((id) => <span key={id} className="mx-crew-stack-av"><CrewAvatar who={whoIs(state, id)} size={20} /></span>)}</span>}
              name={entry.name}
              sub={chatSubtitle(state, entry.id)}
              unread={entry.unread}
              selected={state.selected === entry.id}
              label={`${entry.name}, ${chatSubtitle(state, entry.id)}`}
              onPick={() => { open(entry.id) }}
            />
          ))}
          {adding === 'crew' && <NewCrewForm onCreate={(name) => { setAdding(null); crew.createCrew(name) }} onCancel={() => { setAdding(null) }} />}
        </div>
      </Collapse>
    </div>
  )
}

