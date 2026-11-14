import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'

export function KeyboardDialog({
  open,
  onClose,
  primaryModifier,
}: {
  open: boolean
  onClose: () => void
  primaryModifier: '⌘' | 'Ctrl'
}) {
  const shortcuts = [
    ['Open command switcher', primaryModifier, 'K'],
    ['Start a session in a folder', primaryModifier, 'O'],
    ['Move through switcher results', '↑ or ↓', ''],
    ['Open the focused switcher result', 'Enter', ''],
    ['Open the focused conversation in terminal', primaryModifier, 'Enter'],
    ['Close the command switcher', 'Esc', ''],
    ['Open settings', primaryModifier, ','],
    ['New conversation', primaryModifier, 'N'],
    ['Next conversation needing attention', `${primaryModifier}+Shift`, 'A'],
    ['Show or hide Changes', `${primaryModifier}+Shift`, 'D'],
    ['Previous changed file', '[', ''],
    ['Next changed file', ']', ''],
    ['Show or hide Context', primaryModifier, 'I'],
    ['Collapse or expand sidebar', primaryModifier, '\\'],
    ['Switch between chat and terminal', primaryModifier, '`'],
    ['Stop the current turn', primaryModifier, '.'],
    ['Previous conversation', 'Alt', '↑ or K'],
    ['Next conversation', 'Alt', '↓ or J'],
    ['Allow approval once', 'Y or A', ''],
    ['Decline approval', 'N', ''],
    ['Decline and stop turn', 'Shift', 'N'],
    ['Send message', 'Enter', ''],
    ['Add a new line', 'Shift', 'Enter'],
    ['Rename the focused conversation', 'F2', ''],
    ['Show this shortcut list', '?', ''],
  ]

  return (
    <Dialog
      open={open}
      onOpenChange={(isOpen) => {
        if (!isOpen) onClose()
      }}
    >
      <DialogContent className="keyboard-dialog">
        <DialogHeader>
          <DialogTitle>Keyboard shortcuts</DialogTitle>
          <DialogDescription>
            Shortcuts available in Maitty Desktop.
          </DialogDescription>
        </DialogHeader>
        <div className="keyboard-shortcut-list">
          {shortcuts.map(([action, modifier, key]) => (
            <div className="keyboard-shortcut-row" key={action}>
              <span>{action}</span>
              <span>
                {modifier &&
                  modifier
                    .split('+')
                    .map((part) => <kbd key={part}>{part}</kbd>)}
                {key && <kbd>{key}</kbd>}
              </span>
            </div>
          ))}
        </div>
        <DialogFooter>
          <Button variant="soft" onClick={onClose}>
            Close
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
