import { PERCENT, amountWords, percentOf, timeLeftWords, type Transfer } from './downloadEta'

/**
 * How far an update's download is: a bar that fills (and keeps moving while the size is not known yet, so it never
 * looks stuck), what has arrived of what, the percent and the time left.
 */
export function DownloadProgress({ progress, transfer }: { progress: number | null; transfer: Transfer | null }) {
  const percent = percentOf(progress)
  const left = transfer === null ? null : timeLeftWords(transfer)
  const share = percent === null ? null : `${String(percent)}%`
  return (
    <>
      <div className={`mx-upd-bar${percent === null ? ' is-unknown' : ''}`} role="progressbar" aria-label="Download" aria-valuemin={0} aria-valuemax={PERCENT} aria-valuenow={percent ?? undefined}>
        <span style={percent === null ? undefined : { width: `${String(percent)}%` }} />
      </div>
      <p className="mx-upd-progress">
        <span>{transfer === null ? 'Starting…' : amountWords(transfer)}</span>
        <span>{[share, left].filter((part) => part !== null).join(' · ')}</span>
      </p>
    </>
  )
}
