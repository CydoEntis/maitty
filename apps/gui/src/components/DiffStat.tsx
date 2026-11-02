/** "+a −d" in the accent and danger colors. The minus is U+2212, as in the design. */
export function DiffStat({
  additions,
  deletions,
}: {
  additions: number
  deletions: number
}) {
  return (
    <span className="diffstat">
      {additions > 0 && <i>+{additions}</i>}
      {deletions > 0 && (
        <b>
          {'−'}
          {deletions}
        </b>
      )}
    </span>
  )
}
