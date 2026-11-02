/** A menu that opens a dialog must not hand focus back to the row it came from: that would take it from the dialog. */
export const keepFocus = (event: Event) => { event.preventDefault() }
