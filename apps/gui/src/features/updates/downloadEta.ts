/** How far a download is: what has arrived, the whole size when the server said it, and the recent speed once it can be told. */
export type Transfer = { readonly received: number; readonly total: number | null; readonly bytesPerSecond: number | null }

/** One reading of a download: when, and how much had arrived. */
export type Sample = { readonly at: number; readonly received: number }

const MS_PER_SECOND = 1000
const SECONDS_PER_MINUTE = 60
const MINUTES_PER_HOUR = 60
const KB = 1024
const MB = KB * KB
/** From this many megabytes a size is written whole: the tenths no longer matter. */
const WHOLE_FROM_MB = 100
/** The speed is the average over this long, so one stalled or fast moment does not swing the time left. */
export const RATE_WINDOW_MS = 10_000
/** Less than this much history says nothing about the speed yet. */
const RATE_MIN_MS = 2000

/** The readings still inside the window, with the new one added. */
export function keepSamples(samples: readonly Sample[], next: Sample): Sample[] {
  return [...samples.filter((sample) => next.at - sample.at <= RATE_WINDOW_MS), next]
}

/** Bytes a second over the kept readings, or null while there is too little to tell (or nothing arrived). */
export function downloadRate(samples: readonly Sample[]): number | null {
  const first = samples[0]
  const last = samples[samples.length - 1]
  if (first === undefined || last === undefined) return null
  const elapsed = last.at - first.at
  if (elapsed < RATE_MIN_MS) return null
  const rate = ((last.received - first.received) * MS_PER_SECOND) / elapsed
  return rate > 0 ? rate : null
}

/** A size as people say it: `512 KB`, `34.9 MB`. */
export function sizeWords(bytes: number): string {
  if (bytes < MB) return `${String(Math.max(1, Math.round(bytes / KB)))} KB`
  const megabytes = bytes / MB
  return `${megabytes < WHOLE_FROM_MB ? megabytes.toFixed(1) : String(Math.round(megabytes))} MB`
}

/** The time left in words, or null when the size or the speed is not known. */
export function timeLeftWords({ received, total, bytesPerSecond }: Transfer): string | null {
  if (total === null || bytesPerSecond === null || received >= total) return null
  const minutes = Math.round((total - received) / bytesPerSecond / SECONDS_PER_MINUTE)
  if (minutes < 1) return 'less than a minute left'
  if (minutes < MINUTES_PER_HOUR) return `about ${String(minutes)} min left`
  const hours = Math.floor(minutes / MINUTES_PER_HOUR)
  const rest = minutes % MINUTES_PER_HOUR
  return rest === 0 ? `about ${String(hours)} h left` : `about ${String(hours)} h ${String(rest)} min left`
}

/** What has arrived of what: `34.9 MB of 102 MB`, or only the first while the size is unknown. */
export function amountWords({ received, total }: Transfer): string {
  return total === null ? sizeWords(received) : `${sizeWords(received)} of ${sizeWords(total)}`
}
