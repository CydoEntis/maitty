import { useEffect, useState } from 'react'
import { X } from 'lucide-react'
import { attachmentImage } from '@/api/attachments'
import { Dialog, DialogClose, DialogContent, DialogTitle, DialogTrigger } from '@/components/ui/dialog'

/** The picture of each kept file already read, by path: a chat that scrolls back asks the host once. */
const pictures = new Map<string, string>()

/**
 * A kept image as something an `<img>` can show: its address once the host has read it, null while
 * it is being read and when it cannot be (swept, or not an image).
 */
function useKeptPicture(path: string, wanted: boolean): string | null {
  const [, setRead] = useState(0)
  useEffect(() => {
    if (!wanted || pictures.has(path)) return
    let current = true
    attachmentImage(path)
      .then((blob) => {
        pictures.set(path, URL.createObjectURL(blob))
        if (current) setRead((count) => count + 1)
      })
      .catch((error: unknown) => { console.warn('Could not show the attached image', error) })
    return () => { current = false }
  }, [path, wanted])
  return wanted ? (pictures.get(path) ?? null) : null
}

const THUMB = 'size-8 shrink-0 rounded object-cover'

type Props = {
  name: string
  /** The kept file, read by the host when no `preview` is already held. */
  path: string
  /** An address the window already has for the picture (a pasted image's thumbnail). */
  preview?: string | null
  /** The picture's thumbnail when shown, or a stand-in while there is none. */
  fallback: React.ReactNode
  className?: string
  /** A label shown beside the thumbnail and part of what opens the picture, so the whole chip is one click. */
  children?: React.ReactNode
}

/** An image's thumbnail, with its label if given; clicking any of it opens the picture full size, and Escape or a click outside closes it. */
export function ImageThumb({ name, path, preview = null, fallback, className = THUMB, children }: Props) {
  const kept = useKeptPicture(path, preview === null)
  const src = preview ?? kept
  if (src === null) return <>{fallback}{children}</>
  return (
    <Dialog>
      <DialogTrigger asChild>
        <button type="button" aria-label={`View ${name}`} title={`View ${name}`} className="flex min-w-0 shrink-0 cursor-zoom-in items-center gap-2 rounded text-left focus-visible:outline-2 focus-visible:outline-[var(--accent-line)]">
          <img src={src} alt="" className={className} />
          {children}
        </button>
      </DialogTrigger>
      <DialogContent unstyled overlayClassName="fixed inset-0 z-50 grid place-items-center bg-black/70 p-6" aria-describedby={undefined} className="relative flex max-h-full max-w-full outline-none">
        <DialogTitle className="sr-only">{name}</DialogTitle>
        <img src={src} alt={name} className="max-h-[calc(100vh-3rem)] max-w-[calc(100vw-3rem)] rounded-lg object-contain" />
        <DialogClose aria-label="Close" className="absolute -top-3 -right-3 inline-flex size-7 items-center justify-center rounded-full border border-line2 bg-term text-fg2 hover:text-fg">
          <X className="size-4" aria-hidden="true" />
        </DialogClose>
      </DialogContent>
    </Dialog>
  )
}
