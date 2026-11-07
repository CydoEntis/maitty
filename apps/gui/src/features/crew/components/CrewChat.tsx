import { useEffect, useRef, useState, type KeyboardEvent } from 'react'
import { Link2, UserPlus } from 'lucide-react'
import { toast } from 'sonner'
import { Icon } from '@/features/workspace/components/glyphs'
import type { CrewApi } from '../useCrew'
import { chatSubtitle, isCrew, mateAgentWord, notAboard, whoIs, type CrewState } from '../model/crew'
import { CrewAvatar } from './CrewAvatar'

const STACK_LIMIT = 4

function Header({ crew, chat }: { crew: CrewApi; chat: string }) {
  const { state } = crew
  const [picking, setPicking] = useState(false)
  const entry = state.crews.find((candidate) => candidate.id === chat)
  const mate = state.mates.find((candidate) => candidate.id === chat)
  const choices = notAboard(state, chat)
  const name = entry?.name ?? mate?.name ?? ''
  return (
    <div className="mx-crew-head">
      {entry && <span className="mx-crew-stack is-head" aria-hidden="true">{entry.members.slice(0, STACK_LIMIT).map((id) => <span key={id} className="mx-crew-stack-av"><CrewAvatar who={whoIs(state, id)} size={26} /></span>)}</span>}
      {mate && <CrewAvatar who={mate} size={32} status={mate.status} />}
      <span className="mx-crew-head-text">
        <span className="mx-crew-head-name">{name}</span>
        <span className="mx-crew-head-sub">{chatSubtitle(state, chat)}</span>
      </span>
      {entry && (
        <div className="mx-crew-head-actions">
          <button type="button" className="mx-crew-pill is-strong" aria-haspopup="menu" aria-expanded={picking} onClick={() => { setPicking((on) => !on) }}><UserPlus size={13} aria-hidden="true" />Add crewmate</button>
          <button type="button" className="mx-crew-pill" onClick={() => { toast.message('Invites aren’t available yet', { description: 'Crews with other people need a sync service, which isn’t built.' }) }}><Link2 size={13} aria-hidden="true" />Invite</button>
          {picking && (
            <>
              <div className="mx-crew-scrim" onClick={() => { setPicking(false) }} />
              <div className="mx-crew-menu" role="menu" aria-label="Your crewmates" onKeyDown={(event) => { if (event.key === 'Escape') { event.stopPropagation(); setPicking(false) } }}>
                <div className="mx-crew-menu-label">YOUR CREWMATES</div>
                {choices.map((choice) => (
                  <button key={choice.id} type="button" role="menuitem" className="mx-crew-menu-item" onClick={() => { crew.bringAboard(chat, choice.id); setPicking(false) }}>
                    <CrewAvatar who={choice} size={22} status={choice.status} />
                    <span className="mx-crew-menu-name">{choice.name}</span>
                    <span className="mx-crew-menu-agent">{mateAgentWord(choice.agent)}</span>
                  </button>
                ))}
                {choices.length === 0 && <div className="mx-crew-menu-none">All your crewmates are aboard.</div>}
              </div>
            </>
          )}
        </div>
      )}
    </div>
  )
}

function Lines({ state, chat }: { state: CrewState; chat: string }) {
  const end = useRef<HTMLDivElement>(null)
  const lines = state.lines[chat] ?? []
  const typing = state.typing?.chat === chat ? whoIs(state, state.typing.who) : null
  useEffect(() => { end.current?.scrollIntoView({ block: 'end' }) }, [lines.length, typing?.id])
  return (
    <div className="mx-crew-scroll">
      <div className="mx-crew-column">
        {lines.length === 0 && <div className="mx-crew-empty">Nothing here yet. Say hello.</div>}
        {lines.map((line, index) => {
          if (line.kind === 'system') return <div key={index} className="mx-crew-system"><span />{line.text}<span /></div>
          if (line.from === 'you') return <div key={index} className="mx-crew-you"><div className="mx-crew-bubble">{line.text}</div></div>
          const who = whoIs(state, line.from)
          const captain = state.mates[0]?.id === who.id
          return (
            <div key={index} className="mx-crew-other">
              <CrewAvatar who={who} size={28} status={state.mates.find((mate) => mate.id === who.id)?.status} />
              <div className="mx-crew-other-body">
                <div className="mx-crew-other-head">
                  <span className="mx-crew-other-name">{who.name}</span>
                  {captain && <span className="mx-crew-tag">CAPTAIN</span>}
                  <span className="mx-crew-time">{line.time}</span>
                </div>
                <div className="mx-crew-text-body">{line.text}</div>
              </div>
            </div>
          )
        })}
        {typing && (
          <div className="mx-crew-typing" role="status">
            <CrewAvatar who={typing} size={28} />
            <span>{typing.name} is typing</span>
            <span className="mx-crew-dots" aria-hidden="true"><i /><i /><i /></span>
          </div>
        )}
        <div ref={end} />
      </div>
    </div>
  )
}

function Composer({ crew, chat }: { crew: CrewApi; chat: string }) {
  const { state } = crew
  const [drafts, setDrafts] = useState<Readonly<Record<string, string>>>({})
  const draft = drafts[chat] ?? ''
  const box = useRef<HTMLTextAreaElement>(null)
  useEffect(() => { box.current?.focus() }, [chat])
  const submit = () => {
    if (draft.trim() === '') return
    crew.say(chat, draft)
    setDrafts((current) => ({ ...current, [chat]: '' }))
  }
  const onKeyDown = (event: KeyboardEvent) => {
    if (event.key !== 'Enter' || event.shiftKey) return
    event.preventDefault()
    submit()
  }
  const target = isCrew(state, chat) ? 'the crew' : whoIs(state, chat).name
  return (
    <div className="mx-crew-composer">
      <div className="mx-crew-well">
        <textarea
          ref={box}
          rows={1}
          value={draft}
          aria-label="Message"
          placeholder={isCrew(state, chat) ? `Message ${target} · @name asks one crewmate` : `Message ${target}…`}
          onChange={(event) => { const value = event.target.value; setDrafts((current) => ({ ...current, [chat]: value })) }}
          onKeyDown={onKeyDown}
        />
        <button type="button" className="mx-crew-send" aria-label="Send" title="Send (Enter)" disabled={draft.trim() === ''} onClick={submit}><Icon name="arrowUp" size={15} /></button>
      </div>
      <div className="mx-crew-composer-hint">Enter sends · Shift+Enter adds a line</div>
    </div>
  )
}

/** A crewmate's or a crew's chat. It covers the workspace card, and the tab bar is hidden under it. */
export function CrewChat({ crew }: { crew: CrewApi }) {
  const chat = crew.state.selected
  const name = crew.state.crews.find((entry) => entry.id === chat)?.name ?? whoIs(crew.state, chat).name
  return (
    <div className="mx-crew-chat" role="region" aria-label={name}>
      <Header crew={crew} chat={chat} />
      <Lines state={crew.state} chat={chat} />
      <Composer crew={crew} chat={chat} />
    </div>
  )
}
