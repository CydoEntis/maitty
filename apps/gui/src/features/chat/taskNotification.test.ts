import { describe, expect, it } from 'vitest'
import { parseTaskNotifications } from './taskNotification'

const BLOCK = `<task-notification>
<task-id>a78</task-id>
<status>completed</status>
<summary>Agent "Sleep 3 seconds C" finished</summary>
<note>A task-notification fires each time this agent stops.</note>
<usage><subagent_tokens>50506</subagent_tokens><tool_uses>2</tool_uses><duration_ms>11312</duration_ms></usage>
</task-notification>`

describe('task notifications', () => {
  it('reads a helper finishing as one line with its time and tokens, and none of the markup', () => {
    expect(parseTaskNotifications(BLOCK)).toEqual(['Agent "Sleep 3 seconds C" finished · 11s · 50.5k tokens'])
  })

  it('gives each notification in a message its own line', () => {
    expect(parseTaskNotifications(`${BLOCK}\n${BLOCK}`)).toHaveLength(2)
  })

  it('leaves what a person wrote alone, even when it mentions the tag later', () => {
    expect(parseTaskNotifications('what is a <task-notification>?')).toBeNull()
    expect(parseTaskNotifications('hello')).toBeNull()
  })

  it('names the status when there is no summary', () => {
    expect(parseTaskNotifications('<task-notification><status>failed</status></task-notification>')).toEqual(['Agent failed'])
  })
})
