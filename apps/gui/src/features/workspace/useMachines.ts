import { useCallback, useEffect, useRef, useState, type RefObject } from 'react'
import { z } from 'zod'
import { conversationsFromSnapshot, isDesktop } from '@/api/daemon'
import { LOCAL_MACHINE, addMachine, cancelAddMachine, listenMachineClipboard, listenMachineError, listenMachineNotice, listenMachineRemoved, listenMachineSessions, listenMachineState, listenMachineUpdateProgress, loadMachinesOverview, removeMachine, retryMachine, setMachineGrant, terminalOf, unregisterMachine, type Grant, type MachineState } from '@/api/machines'
import type { Conversation } from '@/data/preview'
import { sendNativeNotice } from '@/features/notifications/native'
import { COPY_TOAST_ID, COPY_TOAST_MS, copySaid } from '@/features/terminal/copyToast'
import { errorMessage } from '@/lib/error-message'
import { toast } from '@/lib/toast'
import { runMachineUpdate } from './machineUpdate'
import { advanceLoginWatch, LOGIN_CHECK_MS, startLoginWatch, type LoginWatch } from './model/loginWatch'
import { announcementFor, type RemoteMachine, type UpdateAction } from './model/machines'
import { copyText } from '@/lib/clipboard'

/** The other machines as the host reports them, and what was last said about one for a screen reader. */
export type RemoteMachines = {
  machines: ReadonlyMap<number, RemoteMachine>
  /** The same map, for a callback that outlives the render it was made in. */
  machinesRef: RefObject<ReadonlyMap<number, RemoteMachine>>
  /** The first answer about the machines has come, so a Machines page can tell loading from empty. */
  machinesKnown: boolean
  machineSaid: string
}

export function useRemoteMachines(): RemoteMachines {
  // Other machines (ADR 0007): the host holds their connections and tells the window each one's state
  // and, when it changes, the sessions it lists. The window never connects.
  const [machines, setMachines] = useState<ReadonlyMap<number, RemoteMachine>>(new Map())
  const machinesRef = useRef(machines)
  useEffect(() => { machinesRef.current = machines }, [machines])
  // The first answer about the machines has come, so a Machines page can tell loading from empty.
  const [machinesKnown, setMachinesKnown] = useState(false)
  // Said to a screen reader when a machine goes down or comes back.
  const [machineSaid, setMachineSaid] = useState('')
  useEffect(() => {
    const stops: Array<() => void> = []
    let gone = false
    const keep = (stop: () => void) => { if (gone) stop(); else stops.push(stop) }
    const change = (id: number, update: (before: RemoteMachine | undefined) => RemoteMachine | undefined) => {
      setMachines((current) => {
        const next = update(current.get(id))
        if (next === undefined) return current
        return new Map(current).set(id, next)
      })
    }
    // The host names a machine in its state, and only by number in its snapshots.
    const names = new Map<number, string>()
    const settle = (state: MachineState) => {
      if (state.machine === LOCAL_MACHINE) return
      names.set(state.machine, state.name)
      const said = announcementFor(machinesRef.current.get(state.machine)?.state, state)
      if (said !== null) setMachineSaid(said)
      change(state.machine, (before) => ({ conversations: before?.conversations ?? [], heard: before?.heard ?? false, state }))
      // Typing the machine could not take while it was down: say so once it is back.
      if (state.state === 'connected' && state.unsent) {
        toast.message(`${String(state.unsent)} ${state.unsent === 1 ? 'keystroke' : 'keystrokes'} to ${state.name} ${state.unsent === 1 ? 'was' : 'were'} not sent while it was disconnected`)
      }
    }
    const list = (id: number, name: string, snapshot: unknown) => {
      let conversations: Conversation[]
      try {
        conversations = conversationsFromSnapshot(snapshot, { id, name })
      } catch (error: unknown) {
        if (!(error instanceof z.ZodError)) throw error
        console.warn(`${name} sent a session list this window cannot read`, error)
        return
      }
      change(id, (before) => (before ? { ...before, conversations, heard: true } : undefined))
    }
    void loadMachinesOverview().then((overview) => {
      if (gone) return
      for (const machine of overview) {
        settle(machine)
        if (machine.snapshot !== null) list(machine.machine, machine.name, machine.snapshot)
      }
      setMachinesKnown(true)
    }, (error: unknown) => {
      console.warn('Could not read the state of the machines', error)
      if (!gone) setMachinesKnown(true)
    })
    for (const listening of [
      listenMachineState(settle),
      listenMachineError((text) => { toast.error(text) }),
      // A machine that was allowed to raise notices: in the window as a toast, away from it as the system's.
      // Text a program in a pane copied: this computer's, or a machine's that was allowed to.
      listenMachineClipboard((id, text) => {
        const said = copySaid(id, names.get(id) ?? 'A machine', text.length)
        copyText(text).then(
          () => { toast.success(said.done, { id: COPY_TOAST_ID, duration: COPY_TOAST_MS }) },
          (error: unknown) => { toast.error(errorMessage(error, said.failed)) },
        )
      }),
      listenMachineNotice((text) => {
        if (document.hasFocus()) toast.message(text)
        else sendNativeNotice(text, '').catch((error: unknown) => { console.warn('Could not show a system notification', error) })
      }),
      listenMachineRemoved((id) => {
        const name = names.get(id)
        names.delete(id)
        if (name !== undefined) unregisterMachine(name)
        setMachines((current) => {
          if (!current.has(id)) return current
          const next = new Map(current)
          next.delete(id)
          return next
        })
      }),
      listenMachineSessions((id, snapshot) => {
        const name = names.get(id)
        if (snapshot !== null && name !== undefined) list(id, name, snapshot)
      }),
    ]) {
      listening.then(keep, (error: unknown) => { toast.error(errorMessage(error, 'Could not follow the machines')) })
    }
    return () => {
      gone = true
      for (const stop of stops) stop()
    }
  }, [])
  return { machines, machinesRef, machinesKnown, machineSaid }
}

/** What the window can ask of a machine by its name: add, retry, change what it may do, update, remove. */
export function useMachineActions(localConversations: readonly Conversation[] | undefined) {
  const retryNamed = useCallback((name: string) => {
    retryMachine(name).catch((error: unknown) => { toast.error(errorMessage(error, `Could not retry ${name}`)) })
  }, [])
  // A first login runs in a session on this computer; the machine is dialled again when it closes.
  const localTermsRef = useRef<ReadonlySet<number>>(new Set())
  const loginsRef = useRef<LoginWatch[]>([])
  const [watchingLogins, setWatchingLogins] = useState(false)
  const checkLogins = useCallback(() => {
    const next: LoginWatch[] = []
    for (const watch of loginsRef.current) {
      const step = advanceLoginWatch(watch, localTermsRef.current, Date.now())
      if (step.dial) retryNamed(watch.machine)
      if (step.watch) next.push(step.watch)
    }
    loginsRef.current = next
    setWatchingLogins(next.length > 0)
  }, [retryNamed])
  useEffect(() => {
    const terms = (localConversations ?? []).filter((item) => item.kind === 'terminal').map((item) => terminalOf(item.id))
    localTermsRef.current = new Set(terms.filter((term): term is number => term !== null))
  }, [localConversations])
  useEffect(() => {
    if (!watchingLogins) return
    const timer = setInterval(checkLogins, LOGIN_CHECK_MS)
    return () => { clearInterval(timer) }
  }, [watchingLogins, checkLogins])
  const addNamed = useCallback(async (target: string, name?: string) => {
    const added = await addMachine(target, name)
    if (added.outcome === 'login') {
      loginsRef.current = [...loginsRef.current, startLoginWatch(added.name, localTermsRef.current, Date.now())]
      setWatchingLogins(true)
    }
    return added
  }, [])
  const grantNamed = useCallback((name: string, grant: Grant, on: boolean) => {
    setMachineGrant(name, grant, on).catch((error: unknown) => { toast.error(errorMessage(error, `Could not change ${name}`)) })
  }, [])
  // The restarted daemon is dialled again once the update is in: the state moves on by itself.
  // One update per machine at a time: two would race to replace the same file there.
  const updatingRef = useRef<Set<string>>(new Set())
  // How far each machine's update has got. A machine is in it from the moment its update starts until it ends.
  const [updateProgress, setUpdateProgress] = useState<Readonly<Record<string, number>>>({})
  useEffect(() => {
    if (!isDesktop()) return undefined
    let stopped = false
    let unlisten: (() => void) | undefined
    listenMachineUpdateProgress((progress) => {
      setUpdateProgress((current) => (progress.name in current ? { ...current, [progress.name]: progress.percent } : current))
    }).then((stop) => { if (stopped) stop(); else unlisten = stop }, (error: unknown) => { console.warn('Could not listen for update progress', error) })
    return () => { stopped = true; unlisten?.() }
  }, [])
  const updateNamed = useCallback(async (name: string, action: UpdateAction, whenIdle: boolean) => {
    if (updatingRef.current.has(name)) {
      toast.message(`${name} is already being updated.`)
      return
    }
    updatingRef.current.add(name)
    setUpdateProgress((current) => ({ ...current, [name]: 0 }))
    try {
      await runMachineUpdate(name, action, whenIdle, () => { retryNamed(name) })
    } finally {
      updatingRef.current.delete(name)
      setUpdateProgress((current) => Object.fromEntries(Object.entries(current).filter(([key]) => key !== name)))
    }
  }, [retryNamed])
  const copyCommand = useCallback((command: string) => {
    copyText(command).then(
      () => { toast.message('Copied. Run it in a terminal on this computer.') },
      (error: unknown) => { toast.error(errorMessage(error, 'Could not copy the command')) },
    )
  }, [])
  const cancelAdding = useCallback(() => {
    cancelAddMachine().catch((error: unknown) => { toast.error(errorMessage(error, 'Could not cancel')) })
  }, [])
  const removeNamed = useCallback((name: string) => {
    removeMachine(name).then(
      () => { toast.message(`Removed ${name}. Its sessions keep running on ${name}.`) },
      (error: unknown) => { toast.error(errorMessage(error, `Could not remove ${name}`)) },
    )
  }, [])
  return { retryNamed, addNamed, grantNamed, updateNamed, updateProgress, copyCommand, cancelAdding, removeNamed }
}
