/** A neutral folder outline with the group's letters inside, for the collapsed rail. */
export function RailFolder({ initials, focused }: { initials: string; focused: boolean }) {
  return (
    <span className={`mx-rail-folder${focused ? ' is-focused' : ''}`} aria-hidden="true">
      <svg viewBox="0 0 36 28" width="30" height="26" focusable="false">
        <path className="mx-rail-folder-shape" d="M2 6.5A3.5 3.5 0 0 1 5.5 3H13l3 3.5h14.5A3.5 3.5 0 0 1 34 10v14.5a3 3 0 0 1-3 3H5a3 3 0 0 1-3-3z" />
      </svg>
      <span className="mx-rail-folder-text">{initials}</span>
    </span>
  )
}
