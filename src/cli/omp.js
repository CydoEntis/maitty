// Written by `seshi integrate omp`: lifecycle adapted from Herdr 143bf5fd4781d164260c63fe28e7dc08054ad32f.
// SPDX-License-Identifier: Apache-2.0; see vendor/herdr-detection/NOTICE.md.
// Local changes: authenticated CLI transport, fixed timings and no prompt text forwarding.
import { execFile } from "node:child_process";
import { promisify } from "node:util";
const execute = promisify(execFile);
const executable = __SESHI_EXE__;
const IDLE_DELAY_MS = 250;
const RETRY_GRACE_MS = 2500;
const retryable = /overloaded|provider.?returned.?error|rate.?limit|too many requests|429|500|502|503|504|service.?unavailable|server.?error|internal.?error|network.?error|connection.?error|connection.?refused|connection.?lost|websocket.?closed|websocket.?error|other side closed|fetch failed|upstream.?connect|reset before headers|socket hang up|ended without|http2 request did not get a response|timed? out|timeout|terminated|retry delay/i;

export default function (pi) {
  if (!process.env.SESHI_TERM_ID || process.env.OMPCODE === "1") return;
  let root = false, active = false, retrying = false, failed = false, blockers = 0;
  let idleTimer, retryTimer, last, warned = false, ctx, generation = 0;
  let queue = Promise.resolve();
  const clear = () => { clearTimeout(idleTimer); clearTimeout(retryTimer); };
  const activate = (context) => {
    if (!context?.hasUI) return false;
    ctx = context;
    root = true;
    return true;
  };
  const report = (force = false) => {
    const status = blockers || failed ? "blocked" : active || retrying ? "working" : last === "working" || last === "blocked" ? "done" : "idle";
    if (!force && status === last) return queue;
    last = status;
    const revision = generation;
    const payload = JSON.stringify({ hook_event_name: "omp.lifecycle", cwd: ctx?.cwd, session_id: ctx?.sessionManager?.getSessionId?.() });
    queue = queue.then(async () => {
      if (!root || revision !== generation) return;
      try {
        await execute(executable, ["hook", "omp", payload, "--status", status], { timeout: 2000, windowsHide: true });
        warned = false;
      } catch {
        last = undefined;
        if (!warned) ctx?.ui?.notify?.("Seshi status report failed. Check that Seshi and its daemon are up to date.", "warning");
        warned = true;
      }
    });
    return queue;
  };
  const start = (context, switched = false) => {
    if (!activate(context)) return;
    generation++;
    clear();
    blockers = 0; retrying = false; failed = false; last = undefined;
    active = !switched && context.isIdle?.() === false;
    return report(true);
  };
  const block = (present) => {
    clear();
    blockers = present ? blockers + 1 : Math.max(0, blockers - 1);
    return report();
  };
  pi.events?.on("herdr:blocked", (data) => root && block(!!data?.active));
  pi.on("session_start", (_, context) => start(context));
  pi.on("session_switch", (_, context) => start(context, true));
  pi.on("agent_start", (_, context) => {
    if (!activate(context)) return;
    clear(); retrying = false; failed = false; active = true;
    return report();
  });
  for (const [event, present] of [["tool_approval_requested", true], ["tool_approval_resolved", false], ["tool_execution_start", true], ["tool_execution_end", false]]) {
    pi.on(event, (data, context) => {
      if (event.startsWith("tool_execution") && data.toolName !== "ask") return;
      if (!root && !activate(context)) return;
      return block(present);
    });
  }
  pi.on("agent_end", (event) => {
    if (!root || !active || event?.willContinue === true) return;
    active = false;
    clear();
    const assistant = [...(event?.messages ?? [])].reverse().find((m) => m.role === "assistant");
    if (assistant?.stopReason === "error" && retryable.test(assistant.errorMessage ?? "")) {
      retrying = true;
      void report();
      retryTimer = setTimeout(() => { retrying = false; failed = true; void report(); }, RETRY_GRACE_MS);
      retryTimer.unref?.();
    } else {
      retrying = false; failed = false;
      idleTimer = setTimeout(() => { void report(); }, IDLE_DELAY_MS);
      idleTimer.unref?.();
    }
  });
  pi.on("session_shutdown", () => { clear(); root = false; generation++; });
}
