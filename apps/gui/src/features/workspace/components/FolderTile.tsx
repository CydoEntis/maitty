import { folderStyle } from './folderStyle'

/** A two-tone folder in the group's colour with its initials on the front, for the collapsed rail. */
export function RailFolder({ colorKey, colorId, initials }: { colorKey: string; colorId?: string | undefined; initials: string }) {
  return (
    <span className="mx-rail-folder" style={folderStyle(colorKey, colorId)} aria-hidden="true">
      <svg viewBox="0 0 36 28" width="34" height="26" focusable="false">
        <path className="mx-rail-folder-back" d="M2 6.5A3.5 3.5 0 0 1 5.5 3H13l3 3.5h14.5A3.5 3.5 0 0 1 34 10v14.5a3 3 0 0 1-3 3H5a3 3 0 0 1-3-3z" />
        <path className="mx-rail-folder-front" d="M2 11.5A2.5 2.5 0 0 1 4.5 9h27a2.5 2.5 0 0 1 2.5 2.5V25a3 3 0 0 1-3 3H5a3 3 0 0 1-3-3z" />
      </svg>
      <span className="mx-rail-folder-text">{initials}</span>
    </span>
  )
}
