// Written by `seshi integrate kilo`: events from Herdr 143bf5fd4781d164260c63fe28e7dc08054ad32f.
// SPDX-License-Identifier: Apache-2.0; see vendor/herdr-detection/NOTICE.md.
// Local changes: authenticated CLI transport, serialized reports and pending-request tracking.
import { execFile } from "node:child_process";
import { promisify } from "node:util";
const execute = promisify(execFile);
const executable = __SESHI_EXE__;

export const Seshi = async ({ client } = {}) => {
  if (!process.env.SESHI_TERM_ID) return {};
  const pending = new Set();
  let queue = Promise.resolve(), warned = false, active, state = "idle";
  const send = (id, next, event) => {
    if (!id || (active && id !== active)) return queue;
    active = id;
    const payload = JSON.stringify({ hook_event_name: event, session_id: id });
    queue = queue.then(async () => {
      try {
        await execute(executable, ["hook", "kilo", payload, "--status", next], { timeout: 2000, windowsHide: true });
        warned = false;
      } catch {
        if (!warned) {
          try {
            await client?.app?.log?.({ body: { service: "seshi", level: "warn", message: "Seshi status report failed; check the daemon version." } });
          } catch {
            console.warn("Seshi status report and Kilo warning log failed; later events will retry.");
          }
        }
        warned = true;
      }
    });
    return queue;
  };
  return {
    "chat.message": async ({ sessionID }) => { active = sessionID; pending.clear(); state = "working"; await send(sessionID, state, "chat.message"); },
    event: async ({ event }) => {
      const props = event?.properties ?? {}, type = event?.type, id = props.sessionID;
      if (!id || (active && active !== id)) return;
      if (["permission.asked", "question.asked"].includes(type)) pending.add(props.id);
      else if (["permission.replied", "question.replied", "question.rejected"].includes(type)) pending.delete(props.requestID ?? props.id);
      else if (type === "session.status") {
        if (["active", "busy", "pending", "running", "streaming", "working"].includes(props.status?.toLowerCase?.())) state = "working";
        else if (props.status?.toLowerCase?.() === "idle") state = state === "working" ? "done" : "idle";
        else return;
      } else if (["tool.execute.before", "tool.execute.after", "session.compacted"].includes(type)) state = "working";
      else if (type === "session.error") state = "blocked";
      else if (type === "session.idle") state = state === "working" ? "done" : "idle";
      else return;
      await send(id, pending.size ? "blocked" : state, type);
    },
  };
};
