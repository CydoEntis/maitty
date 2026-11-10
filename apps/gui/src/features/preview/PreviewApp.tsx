import { usePreviewState } from './usePreviewState'
import { PreviewContext } from './previewContext'
import { PreviewWorkspace } from './PreviewWorkspace'
import { PreviewEmpty } from './PreviewEmpty'

export function PreviewApp() {
  const preview = usePreviewState()
  return (
    <PreviewContext.Provider value={preview}>
      {preview.selected ? <PreviewWorkspace /> : <PreviewEmpty />}
    </PreviewContext.Provider>
  )
}
