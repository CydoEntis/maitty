import { isNotARepository, messageOf } from './useRepoQuery'

/** What a repository tab says when its question failed: for a plain folder, what to do; otherwise the daemon's own words. */
export function RepoError({ error }: { error: unknown }) {
  if (isNotARepository(error)) {
    return (
      <div className="mx-inspect-empty" role="status">
        <strong>This folder is not a git repository.</strong>
        <span>Files, Changes, Branches, History and References read a git repository. Run <code>git init</code> here to make it one, or start a session inside a project that already is.</span>
      </div>
    )
  }
  return <p className="mx-inspect-note is-error" role="alert">{messageOf(error)}</p>
}
