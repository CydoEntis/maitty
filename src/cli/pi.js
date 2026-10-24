// Written by `__APP_COMMAND__ integrate pi`: inert outside __APP_NAME__; requires Pi's agent_settled and ui_prompt events.
import { execFile } from "node:child_process";
import { promisify } from "node:util";

const execute = promisify(execFile);
const executable = __APP_EXE__;

export default function (pi) {
  if (!process.env.__APP_ENV___TERM_ID) return;
  let running = false;
  let prompts = 0;
  let warned = false;
  const report = async (event, ctx, status) => {
    if (ctx.mode && ctx.mode !== "tui") return;
    const payload = { hook_event_name: `pi.${event}`, cwd: ctx.cwd };
    const args = ["hook", "pi", JSON.stringify(payload)];
    if (status) args.push("--status", status);
    try {
      await execute(executable, args, { timeout: 3000, windowsHide: true });
      warned = false;
    } catch {
      if (!warned && ctx.hasUI) ctx.ui.notify("__APP_NAME__ status report failed. Check that __APP_NAME__ and its daemon are up to date.", "warning");
      warned = true;
    }
  };
  pi.on("session_start", async (_, ctx) => {
    running = ctx.isIdle?.() === false;
    prompts = 0;
    await report("session_start", ctx, running ? "working" : "idle");
  });
  pi.on("agent_start", async (_, ctx) => {
    running = true;
    await report("agent_start", ctx, prompts ? "blocked" : "working");
  });
  pi.on("ui_prompt_start", async (_, ctx) => {
    prompts++;
    await report("ui_prompt_start", ctx);
  });
  pi.on("ui_prompt_end", async (_, ctx) => {
    prompts = Math.max(0, prompts - 1);
    await report("ui_prompt_end", ctx, prompts ? "blocked" : running ? "working" : "idle");
  });
  // agent_end can be followed by retries or compaction; only settlement finishes the run.
  pi.on("agent_settled", async (_, ctx) => {
    if (ctx.isIdle?.() === false) return;
    running = false;
    await report("agent_settled", ctx, prompts ? "blocked" : "done");
  });
  pi.on("session_shutdown", async (_, ctx) => report("session_shutdown", ctx));
}
