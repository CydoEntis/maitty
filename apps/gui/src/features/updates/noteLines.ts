const MAX_NOTES = 5

/** The first few lines of a release's notes, without their list markers. */
export function noteLines(notes: string): string[] {
  return notes.split('\n').map((line) => line.replace(/^[-*]\s*/, '').trim()).filter(Boolean).slice(0, MAX_NOTES)
}
