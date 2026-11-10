import { useState } from 'react'
import { Check, CheckCheck, ChevronDown, CircleHelp, ShieldCheck } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { HighlightedCode } from '@/components/HighlightedCode'
import type { Conversation, Message, UserInputQuestion } from '@/data/preview'
import { commandLanguage } from './commandLanguage'
import { PERMISSION_JSON_INDENT_SPACES } from './shared'

export function ApprovalMarker({ message }: { message: Message }) {
  const resolved = message.resolved
  const text = resolved
    ? 'Request answered'
    : message.approvalKind === 'UserInput'
      ? 'Waiting for your answer below'
      : 'Waiting for your approval below'
  return (
    <div
      className={`approval-marker ${resolved ? 'is-resolved' : ''}`}
      role="status"
    >
      <span className="approval-marker-box" aria-hidden="true">
        <i />
      </span>
      <span>{text}</span>
    </div>
  )
}

export function ApprovalDock({
  message,
  provider,
  desktop,
  disabled,
  collapsed,
  onToggle,
  onResolve,
  onUserInput,
}: {
  message: Message
  provider: Conversation['provider']
  desktop: boolean
  disabled: boolean
  collapsed: boolean
  onToggle: () => void
  onResolve: (
    decision: 'Accept' | 'AcceptForSession' | 'Decline' | 'Cancel',
  ) => void
  onUserInput: (answers: Record<string, string[]>) => void
}) {
  const choices =
    message.approvalChoices ??
    (desktop ? [] : ['Decline', 'Accept', 'AcceptForSession', 'Cancel'])
  return (
    <section
      className={`approval-dock ${collapsed ? 'is-collapsed' : ''}`}
      aria-label={
        message.approvalKind === 'UserInput'
          ? 'Question needs an answer'
          : 'Approval needed'
      }
    >
      <div className="approval-dock-heading">
        <button
          className="approval-dock-toggle"
          type="button"
          onClick={onToggle}
          aria-expanded={!collapsed}
        >
          <span className="approval-dock-status">
            <ShieldCheck size={15} />
            {message.approvalKind === 'UserInput'
              ? 'Your answer needed'
              : 'Approval needed'}
          </span>
          <span className="approval-dock-summary">
            {message.approvalKind === 'Command'
              ? message.detail
              : message.content}
          </span>
          <ChevronDown size={15} />
        </button>
        {collapsed &&
          (message.approvalKind === 'UserInput' ? (
            <Button size="sm" onClick={onToggle} disabled={disabled}>
              Answer
            </Button>
          ) : (
            <div className="approval-dock-quick-actions">
              {choices.includes('Decline') && (
                <Button
                  data-approval-choice="Decline"
                  variant="ghost"
                  size="sm"
                  onClick={() => {
                    onResolve('Decline')
                  }}
                  disabled={disabled}
                >
                  Deny
                </Button>
              )}
              {choices.includes('Accept') && (
                <Button
                  data-approval-choice="Accept"
                  size="sm"
                  onClick={() => {
                    onResolve('Accept')
                  }}
                  disabled={disabled}
                >
                  Allow once
                </Button>
              )}
            </div>
          ))}
      </div>
      {disabled && (
        <p className="approval-disconnected" role="status">
          Reconnect to the daemon before answering this request.
        </p>
      )}
      <div className="approval-dock-content" hidden={collapsed}>
        {message.approvalKind === 'UserInput' ? (
          <UserInputRequest
            message={message}
            provider={provider}
            questions={message.userQuestions ?? []}
            onSubmit={onUserInput}
            disabled={disabled}
          />
        ) : (
          <ApprovalRequest
            message={message}
            provider={provider}
            desktop={desktop}
            onResolve={onResolve}
            disabled={disabled}
          />
        )}
      </div>
    </section>
  )
}

function UserInputRequest({
  message,
  provider,
  questions,
  onSubmit,
  disabled,
}: {
  message: Message
  provider: Conversation['provider']
  questions: UserInputQuestion[]
  onSubmit: (answers: Record<string, string[]>) => void
  disabled: boolean
}) {
  const [selected, setSelected] = useState<Record<string, string[]>>({})
  const [other, setOther] = useState<Record<string, string>>({})
  const answers = Object.fromEntries(
    questions.map((question) => {
      const chosen = selected[question.id] ?? []
      const note = other[question.id]?.trim()
      const freeform = note
        ? [
            question.is_other && question.options.length > 0
              ? `user_note: ${note}`
              : note,
          ]
        : []
      return [question.id, [...chosen, ...freeform]]
    }),
  )
  const canSubmit =
    questions.length > 0 &&
    questions.every((question) => (answers[question.id]?.length ?? 0) > 0)

  function toggle(questionId: string, label: string, checked: boolean) {
    setSelected((current) => {
      const values = current[questionId] ?? []
      return {
        ...current,
        [questionId]: checked
          ? [...values, label]
          : values.filter((value) => value !== label),
      }
    })
  }

  return (
    <section
      className="approval-request user-input-request"
      aria-label={`${provider} asks for input`}
    >
      <div className="approval-topline">
        <div className="approval-icon">
          <CircleHelp size={16} />
        </div>
        <div>
          <h2>{message.content}</h2>
          <p>Answer these questions to continue the conversation.</p>
        </div>
        <span className="approval-badge">Needs input</span>
      </div>
      {questions.map((question) => (
        <fieldset className="user-input-question" key={question.id}>
          <legend>
            <span>{question.header}</span>
            {question.is_secret && (
              <span className="secret-label">Private answer</span>
            )}
          </legend>
          <p>{question.question}</p>
          {question.options.map((option) => (
            <label className="user-input-option" key={option.label}>
              <input
                type="checkbox"
                checked={(selected[question.id] ?? []).includes(option.label)}
                onChange={(event) => {
                  toggle(question.id, option.label, event.target.checked)
                }}
                disabled={disabled}
              />
              <span>
                <strong>{option.label}</strong>
                <small>{option.description}</small>
              </span>
            </label>
          ))}
          {(question.is_other || question.options.length === 0) && (
            <label className="user-input-other">
              {question.is_other ? 'Other answer' : 'Your answer'}
              <input
                type={question.is_secret ? 'password' : 'text'}
                autoComplete="off"
                aria-label={`${question.is_other ? 'Other answer' : 'Your answer'} for ${question.header}`}
                value={other[question.id] ?? ''}
                onChange={(event) => {
                  setOther((current) => ({
                    ...current,
                    [question.id]: event.target.value,
                  }))
                }}
                disabled={disabled}
              />
            </label>
          )}
        </fieldset>
      ))}
      {questions.length === 0 ? (
        <div className="approval-resolved">
          This input request has no supported questions. Maitty left it
          unanswered.
        </div>
      ) : (
        <div className="approval-actions user-input-actions">
          <span className="approval-scope">
            Answers are sent only to this {provider} request.
          </span>
          <Button
            size="sm"
            onClick={() => {
              onSubmit(answers)
            }}
            disabled={disabled || !canSubmit}
          >
            Send answers
          </Button>
        </div>
      )}
    </section>
  )
}

function ApprovalRequest({
  message,
  provider,
  desktop,
  onResolve,
  disabled,
}: {
  message: Message
  provider: Conversation['provider']
  desktop: boolean
  onResolve: (
    decision: 'Accept' | 'AcceptForSession' | 'Decline' | 'Cancel',
  ) => void
  disabled: boolean
}) {
  const choices =
    message.approvalChoices ??
    (desktop ? [] : ['Decline', 'Accept', 'AcceptForSession', 'Cancel'])
  return (
    <section
      className={`approval-request ${message.resolved ? 'is-resolved' : ''}`}
      aria-label="Approval request"
    >
      <div className="approval-topline">
        <div className="approval-icon">
          <ShieldCheck size={16} />
        </div>
        <div>
          <h2>{message.content}</h2>
          <p>
            {message.approvalReason ??
              (message.approvalKind === 'FileChange'
                ? `Review the requested file change before allowing ${provider} to continue.`
                : message.approvalKind === 'Permission'
                  ? 'Allowing grants this exact permission profile for this turn only.'
                  : `${provider} wants to run this command in the project folder.`)}
          </p>
        </div>
        <span className="approval-badge">
          {message.resolved ? 'Resolved' : 'Needs approval'}
        </span>
      </div>
      {message.approvalKind === 'Command' && (
        <code className="approval-command">
          <HighlightedCode
            code={message.detail ?? ''}
            language={commandLanguage(message.detail)}
          />
        </code>
      )}
      {message.approvalKind === 'FileChange' && message.approvalScope && (
        <p className="approval-scope">
          Requested write access: <code>{message.approvalScope}</code>
        </p>
      )}
      {message.approvalKind === 'Permission' && message.approvalScope && (
        <p className="approval-scope">
          Requested from: <code>{message.approvalScope}</code>
        </p>
      )}
      {message.approvalKind === 'Permission' && message.approvalPermissions && (
        <pre className="approval-permissions">
          {JSON.stringify(
            message.approvalPermissions,
            null,
            PERMISSION_JSON_INDENT_SPACES,
          )}
        </pre>
      )}
      {message.approvalChanges?.map((change) => (
        <section className="approval-change" key={change.path}>
          <strong>{change.path}</strong>
          <pre>{change.diff}</pre>
        </section>
      ))}
      {!message.resolved ? (
        choices.length > 0 ? (
          <div className="approval-actions">
            {choices.includes('Decline') && (
              <Button
                data-approval-choice="Decline"
                variant="ghost"
                size="sm"
                onClick={() => {
                  onResolve('Decline')
                }}
                disabled={disabled}
              >
                Decline
              </Button>
            )}
            <div>
              {choices.includes('Cancel') && (
                <Button
                  data-approval-choice="Cancel"
                  variant="ghost"
                  size="sm"
                  onClick={() => {
                    onResolve('Cancel')
                  }}
                  disabled={disabled}
                >
                  Decline and stop turn
                </Button>
              )}
              {choices.includes('AcceptForSession') && (
                <Button
                  data-approval-choice="AcceptForSession"
                  variant="outline"
                  size="sm"
                  onClick={() => {
                    onResolve('AcceptForSession')
                  }}
                  disabled={disabled}
                >
                  {desktop ? 'Allow this session' : 'Allow session'}
                </Button>
              )}
              {choices.includes('Accept') && (
                <Button
                  data-approval-choice="Accept"
                  size="sm"
                  onClick={() => {
                    onResolve('Accept')
                  }}
                  disabled={disabled}
                >
                  <Check size={14} />
                  Allow once
                </Button>
              )}
            </div>
          </div>
        ) : (
          <div className="approval-resolved">
            {message.approvalKind === 'FileChange' &&
            (message.approvalChanges?.length ?? 0) === 0
              ? `Waiting for ${provider} to send the file diff. Approval is disabled until the requested change is visible.`
              : 'No supported decision is available for this request. Maitty has left it unanswered.'}
          </div>
        )
      ) : (
        <div className="approval-resolved">
          <CheckCheck size={14} />
          {desktop
            ? `Decision sent to ${provider}`
            : 'Decision recorded in local preview'}
        </div>
      )}
    </section>
  )
}

export function RecoveryNotice({
  conversation,
  desktop,
  disabled,
  pending,
  onResume,
}: {
  conversation: Conversation
  desktop: boolean
  disabled: boolean
  pending: boolean
  onResume: () => void
}) {
  const canResumeInChat = desktop && conversation.execution === 'Structured'
  return (
    <div className="recovery-notice" role="alert">
      <CircleHelp size={15} />
      <span>
        {canResumeInChat
          ? `${conversation.provider} stopped before the turn finished. Resume this exact session; Maitty will not replay the interrupted turn.`
          : conversation.execution === 'Native'
            ? 'This terminal session needs recovery. Reopen it from the terminal app.'
            : 'This conversation needs recovery. Reconnect to the daemon before continuing.'}
      </span>
      {canResumeInChat && (
        <Button
          variant="outline"
          size="sm"
          onClick={onResume}
          disabled={disabled}
        >
          {pending ? 'Resuming…' : 'Resume session'}
        </Button>
      )}
    </div>
  )
}
