/** Whether a program other than the plain shell runs in the session, so closing it would stop work. */
export const runsAgent = (process: string): boolean => process !== 'shell'

/** Whether closing this session would stop something: not for one that has already ended. */
export const runsProgram = (session: { process: string; ended: boolean }): boolean => !session.ended && runsAgent(session.process)
