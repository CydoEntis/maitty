/**
 * What the list shows while the history is first read: the shape of a conversation, so the window
 * does not look empty or stuck. Static on purpose: an animation that never stops costs a repaint
 * every frame (docs/PERFORMANCE.md), and the wait is short.
 */
const BLOCK = 'rounded-md bg-raise'

export function ChatSkeleton() {
  return (
    <div role="status" aria-label="Reading the conversation" aria-busy="true" className="mx-auto flex w-full max-w-[760px] flex-1 flex-col gap-5 px-4 py-6">
      <div className={`${BLOCK} ml-auto h-10 w-[42%] rounded-[16px_16px_5px_16px]`} />
      <div className="flex gap-3">
        <div className={`${BLOCK} size-6 shrink-0`} />
        <div className="flex flex-1 flex-col gap-2">
          <div className={`${BLOCK} h-3.5 w-[88%]`} />
          <div className={`${BLOCK} h-3.5 w-[76%]`} />
          <div className={`${BLOCK} h-3.5 w-[52%]`} />
        </div>
      </div>
      <div className={`${BLOCK} ml-9 h-7 w-56`} />
      <div className="flex gap-3">
        <div className={`${BLOCK} size-6 shrink-0`} />
        <div className="flex flex-1 flex-col gap-2">
          <div className={`${BLOCK} h-3.5 w-[64%]`} />
          <div className={`${BLOCK} h-3.5 w-[40%]`} />
        </div>
      </div>
    </div>
  )
}
