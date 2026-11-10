import type { KeyboardEvent } from 'react'
import { ArrowUp, CircleAlert, Square } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Textarea } from '@/components/ui/textarea'
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'
import type { Conversation, SessionStatus } from '@/data/preview'
import { shortcutModifier } from '@/lib/shortcut-modifier'

export function Composer({
  value,
  status,
  conversation,
  desktop,
  connected,
  cancelling,
  deliveryNotice,
  deliveryBlocked,
  onChange,
  onKeyDown,
  onSend,
  denyDecision,
  denyQueued,
  onDenyAndSend,
  onCancel,
  onRetry,
  onDiscard,
  onCheckDelivery,
}: {
  value: string
  status: SessionStatus
  conversation: Conversation
  desktop: boolean
  connected: boolean
  cancelling: boolean
  deliveryNotice: 'not-delivered' | 'unknown' | null
  deliveryBlocked: boolean
  onChange: (value: string) => void
  onKeyDown: (event: KeyboardEvent<HTMLTextAreaElement>) => void
  onSend: () => void
  denyDecision: 'Cancel' | 'Decline' | null
  denyQueued: boolean
  onDenyAndSend: () => void
  onCancel: () => void
  onRetry: () => void
  onDiscard: () => void
  onCheckDelivery: () => void
}) {
  const nativeOwner = conversation.execution === 'Native'
  const denying = denyDecision !== null
  const busy =
    status === 'working' ||
    (status === 'needs-you' && !denying) ||
    nativeOwner ||
    denyQueued
  const composerNote = denyQueued
    ? {
        tone: 'muted' as const,
        text: 'Request denied. Your message sends when this turn ends.',
      }
    : denying
      ? {
          tone: 'muted' as const,
          text: `Typing a message denies the request${denyDecision === 'Cancel' ? ' and stops this turn' : ''}, then sends your instructions instead.`,
        }
      : !connected
        ? {
            tone: 'muted' as const,
            text: 'Your draft is saved. Nothing is sent until you press Send after reconnecting.',
          }
        : status === 'recovery'
          ? {
              tone: 'danger' as const,
              text: 'Resume the session to send messages. Your draft is kept.',
            }
          : nativeOwner
            ? {
                tone: 'muted' as const,
                text: `Native ${conversation.provider} owns this conversation until you continue in chat.`,
              }
            : status === 'working'
              ? {
                  tone: 'muted' as const,
                  text: 'Working · your next message waits until this turn ends',
                }
              : null
  return (
    <div className="composer-wrap">
      {deliveryNotice && (
        <div className={`delivery-notice is-${deliveryNotice}`} role="alert">
          <CircleAlert size={15} />
          <span>
            {deliveryNotice === 'unknown'
              ? 'Delivery is unknown. Maitty will not retry this prompt. Check the native terminal before continuing.'
              : 'Message was not delivered. Your draft is still here and safe to retry.'}
          </span>
          {deliveryNotice === 'not-delivered' ? (
            <>
              <Button
                variant="outline"
                size="sm"
                onClick={onRetry}
                disabled={!connected || busy}
              >
                Retry
              </Button>
              <Button variant="ghost" size="sm" onClick={onDiscard}>
                Discard draft
              </Button>
            </>
          ) : (
            <Button
              variant="outline"
              size="sm"
              onClick={onCheckDelivery}
              disabled={!connected}
            >
              Check delivery
            </Button>
          )}
        </div>
      )}
      {composerNote && (
        <p
          className={`composer-note ${composerNote.tone === 'danger' ? 'is-danger' : ''}`}
          role="status"
        >
          {composerNote.text}
        </p>
      )}
      <div className="composer-box">
        <Textarea
          value={value}
          onChange={(event) => {
            onChange(event.target.value)
          }}
          onKeyDown={onKeyDown}
          placeholder={
            nativeOwner
              ? status === 'recovery'
                ? 'Recover this native session before chatting…'
                : 'Return to chat from the Terminal panel to continue…'
              : !connected
                ? 'Reconnect before sending. Your draft stays here.'
                : denying
                  ? 'Type instructions to deny this request and send them instead…'
                  : busy
                    ? 'Wait for this turn to finish…'
                    : deliveryBlocked
                      ? 'Check the native terminal before continuing.'
                      : `Ask ${conversation.provider} to work on this project…`
          }
          aria-label={`Message ${conversation.provider}`}
          disabled={busy || deliveryBlocked}
          rows={2}
        />
        <div className="composer-actions">
          <div className="composer-context">
            <span className="provider-mini">
              <span className="codex-glyph" aria-hidden="true">
                ●
              </span>
              {conversation.provider} · {conversation.model}
            </span>
            <span>{conversation.permission.split(' · ')[0]}</span>
          </div>
          <div className="composer-send">
            {((desktop && (status === 'working' || status === 'needs-you')) ||
              (!desktop && status === 'working')) && (
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    variant="outline"
                    size="icon-sm"
                    aria-label="Stop"
                    onClick={onCancel}
                    disabled={cancelling}
                  >
                    <Square size={12} fill="currentColor" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent>
                  {cancelling ? 'Stopping…' : 'Stop this turn'} ·{' '}
                  {shortcutModifier(navigator.userAgent)} .
                </TooltipContent>
              </Tooltip>
            )}
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  size="icon-sm"
                  aria-label={denying ? 'Deny and send' : 'Send message'}
                  className={denying ? 'send-deny' : undefined}
                  onClick={denying ? onDenyAndSend : onSend}
                  disabled={
                    !value.trim() || busy || !connected || deliveryBlocked
                  }
                >
                  <ArrowUp />
                </Button>
              </TooltipTrigger>
              <TooltipContent>
                {denying ? 'Deny the request and send' : 'Send'} · Enter
              </TooltipContent>
            </Tooltip>
          </div>
        </div>
      </div>
    </div>
  )
}
