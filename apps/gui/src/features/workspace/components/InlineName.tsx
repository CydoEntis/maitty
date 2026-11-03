import { useEffect, useRef } from 'react'

type Props = {
  value: string
  /** Shown while the name is empty: what the session is called when it has no name of its own. */
  placeholder: string
  label: string
  maxLength: number
  onCommit: (name: string) => void
  onCancel: () => void
}

/** A name typed where it is shown. Enter, or leaving the field, saves; Escape cancels; an empty name clears it. */
export function InlineName({ value, placeholder, label, maxLength, onCommit, onCancel }: Props) {
  const input = useRef<HTMLInputElement>(null)
  const finished = useRef(false)
  // A beat later, so a menu that is still closing and holds focus (a context menu does) cannot take it back.
  useEffect(() => {
    const timer = window.setTimeout(() => {
      input.current?.focus()
      input.current?.select()
    }, 0)
    return () => { window.clearTimeout(timer) }
  }, [])
  const finish = (save: boolean) => {
    if (finished.current) return
    finished.current = true
    if (save) onCommit(input.current?.value.trim() ?? '')
    else onCancel()
  }
  return (
    <input
      ref={input}
      className="mx-inline-name"
      aria-label={label}
      defaultValue={value}
      placeholder={placeholder}
      maxLength={maxLength}
      spellCheck={false}
      // Keys and clicks typed here belong to the field, not to the tab or row around it.
      onKeyDown={(event) => {
        event.stopPropagation()
        if (event.key === 'Enter') {
          event.preventDefault()
          finish(true)
        } else if (event.key === 'Escape') {
          event.preventDefault()
          finish(false)
        }
      }}
      onClick={(event) => { event.stopPropagation() }}
      onMouseDown={(event) => { event.stopPropagation() }}
      onBlur={() => { finish(true) }}
    />
  )
}
