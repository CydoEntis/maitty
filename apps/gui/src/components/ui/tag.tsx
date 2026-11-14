import * as React from "react"
import { cva, type VariantProps } from "class-variance-authority"
import { cn } from "@/lib/utils"

/** The design's Tag: a short mono word in a hairline outline, coloured by what it says (Added, exit 1, round 2). */
const tagVariants = cva(
  "inline-flex h-[18px] flex-none items-center rounded-(--r-badge) px-1.5 font-(family-name:--mono) text-[10.5px] font-semibold whitespace-nowrap",
  {
    variants: {
      tone: {
        neutral: "text-(--fg2) shadow-[inset_0_0_0_1px_var(--line2)]",
        done: "text-(--done) shadow-[inset_0_0_0_1px_color-mix(in_oklab,var(--done)_45%,transparent)]",
        err: "text-(--err) shadow-[inset_0_0_0_1px_color-mix(in_oklab,var(--err)_45%,transparent)]",
        working: "text-(--working) shadow-[inset_0_0_0_1px_color-mix(in_oklab,var(--working)_45%,transparent)]",
      },
    },
    defaultVariants: { tone: "neutral" },
  }
)

function Tag({ className, tone, ...props }: React.ComponentProps<"span"> & VariantProps<typeof tagVariants>) {
  return <span data-slot="tag" className={cn(tagVariants({ tone }), className)} {...props} />
}

/** File-type tags have their own tone, never the needs colour (`--tag-*` in `tokens.css`). */
const TYPE_TONE: Readonly<Record<string, string>> = {
  TS: "var(--tag-ts)", JS: "var(--tag-ts)",
  TSX: "var(--tag-tsx)", JSX: "var(--tag-tsx)",
  JSON: "var(--tag-data)", "{ }": "var(--tag-data)", YML: "var(--tag-data)", YAML: "var(--tag-data)", TOML: "var(--tag-data)",
  SQL: "var(--tag-sql)",
  ENV: "var(--tag-env)",
}

/** The colour a file-type tag is drawn in: grey for any type without its own. */
function typeTone(tag: string): string {
  return TYPE_TONE[tag] ?? "var(--tag-text)"
}

/** The small square type tag before a file's name (TS, MD, …). */
function TypeTag({ tag }: { tag: string }) {
  const tone = typeTone(tag)
  return (
    <span
      data-slot="type-tag"
      className="inline-flex h-4 min-w-[22px] flex-none items-center justify-center rounded-(--r-badge) px-[3px] font-(family-name:--mono) text-[9px] font-bold text-(--tone) shadow-[inset_0_0_0_1px_color-mix(in_oklab,var(--tone)_35%,transparent)] bg-[color-mix(in_oklab,var(--tone)_16%,transparent)]"
      style={{ "--tone": tone } as React.CSSProperties}
    >
      {tag}
    </span>
  )
}

/** A path split where its last name starts: a folder keeps its closing slash with the name. */
function splitPath(path: string): { dir: string; name: string } {
  const slashed = path.replace(/\\/g, "/")
  const cut = slashed.replace(/\/$/, "").lastIndexOf("/")
  return { dir: slashed.slice(0, cut + 1), name: slashed.slice(cut + 1) }
}

type FileBadgeProps = {
  path: string
  /** The type tag, or a node (a folder icon) in its place; nothing for a type without one. */
  tag?: React.ReactNode
  line?: number | null
  /** Opens the file; without it the badge is text, as inside a row that is itself a button. */
  onOpen?: (() => void) | undefined
  className?: string
}

/**
 * The design's FileBadge: the type tag, then the path with its folder dimmed. A long path shortens from the front, so the
 * file's name always shows (the path is laid out right to left inside an isolate, so the ellipsis lands at its start).
 */
function FileBadge({ path, tag, line = null, onOpen, className }: FileBadgeProps) {
  const { dir, name } = splitPath(path)
  const body = (
    <>
      {typeof tag === "string" ? <TypeTag tag={tag} /> : tag}
      <span className="min-w-0 overflow-hidden text-left font-(family-name:--mono) text-[12.5px] font-medium text-ellipsis whitespace-nowrap [direction:rtl]">
        <bdi>
          {dir !== "" && <span className="text-(--dim)">{dir}</span>}
          <span data-slot="file-name">{name}</span>
          {line !== null && <span className="text-(--link)">:{line}</span>}
        </bdi>
      </span>
    </>
  )
  const base = "inline-flex min-w-0 max-w-full items-center gap-[7px] text-(--fg)"
  return onOpen === undefined
    ? <span data-slot="file-badge" className={cn(base, className)} title={path}>{body}</span>
    : <button type="button" data-slot="file-badge" className={cn(base, "rounded-(--r-badge) outline-none hover:[&_[data-slot=file-name]]:underline focus-visible:shadow-[0_0_0_2px_var(--focus)]", className)} title={path} onClick={onOpen}>{body}</button>
}

/** Inline code in running text. */
function CodeChip({ className, ...props }: React.ComponentProps<"code">) {
  return <code data-slot="code-chip" className={cn("rounded-(--r-badge) bg-(--sel) px-[5px] py-px font-(family-name:--mono) text-[.9em] text-(--fg)", className)} {...props} />
}

/** A link in running text: the link colour with a soft underline. */
function TextLink({ className, ...props }: React.ComponentProps<"a">) {
  return <a data-slot="text-link" className={cn("text-(--link) underline decoration-[color-mix(in_oklab,var(--link)_45%,transparent)] underline-offset-2 hover:decoration-(--link) focus-visible:outline-2 focus-visible:outline-(--focus)", className)} {...props} />
}

export { Tag, tagVariants, TypeTag, typeTone, FileBadge, splitPath, CodeChip, TextLink }
