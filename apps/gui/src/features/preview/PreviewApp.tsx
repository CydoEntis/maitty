import { usePreviewState } from './usePreviewState'
import { PreviewContext } from './previewContext'
import { PreviewWorkspace } from './PreviewWorkspace'
import { PreviewEmpty } from './PreviewEmpty'
import { ServiceBehindNotice } from '@/features/updates/ServiceBehindNotice'

export function PreviewApp() {
  const preview = usePreviewState()
  return (
    <PreviewContext.Provider value={preview}>
      {preview.selected ? <PreviewWorkspace /> : <PreviewEmpty />}
      <ServiceBehindNotice sessionsQuery={preview.sessionsQuery} />
    </PreviewContext.Provider>
  )
}
