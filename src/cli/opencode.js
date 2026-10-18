// Written by `seshi integrate opencode`: inert outside Seshi.
import { execFile } from "node:child_process";
import { promisify } from "node:util";

const execute = promisify(execFile);
const executable = __SESHI_EXE__;

export const Seshi = async ({ directory, client }) => {
  const args = process.argv?.slice(2) ?? [];
  if (args.some((arg) => ["serve", "web", "attach"].includes(arg) || arg === "--attach" || arg.startsWith("--attach="))) return {};
  // Interactive sessions report through their own TUI, including shared servers.
  if (!args.includes("run") && !args.includes("--mini")) return {};
  let queue = Promise.resolve();
  let warned = false;
  const pending = new Set();
  const sessions = new Map();
  let active;
  let rootStatus = "working";
  const relevant = new Set(["session.status", "session.idle", "session.error", "permission.asked", "permission.updated", "permission.replied", "question.asked", "question.replied", "question.rejected"]);
  const rootOf = async (id) => {
    const visited = new Set();
    while (id && !visited.has(id)) {
      visited.add(id);
      if (!sessions.has(id)) {
        const result = await client.session.get({ path: { id }, sessionID: id });
        if (!result.data) throw new Error("session lookup failed");
        sessions.set(id, result.data.parentID ?? null);
      }
      const parent = sessions.get(id);
      if (!parent) return id;
      id = parent;
    }
    throw new Error("invalid session ancestry");
  };
  const send = async (event) => {
    const props = event.properties ?? {};
    if (!props.sessionID) return;
    const root = await rootOf(props.sessionID);
    const isRoot = props.sessionID === root;
    const busy = event.type === "session.status" && ["busy", "retry"].includes(props.status?.type);
    if (!active || (isRoot && busy && root !== active)) {
      active = root;
      pending.clear();
      rootStatus = "working";
    }
    if (root !== active) return;
    const id = props.id ?? props.requestID;
    const key = `${props.sessionID}:${id}`;
    const asked = ["permission.asked", "permission.updated", "question.asked"].includes(event.type);
    const replied = ["permission.replied", "question.replied", "question.rejected"].includes(event.type);
    const ended = event.type === "session.idle" || event.type === "session.error" || (event.type === "session.status" && props.status?.type === "idle");
    if (asked) pending.add(key);
    if (replied) pending.delete(key);
    if (ended) {
      for (const request of pending) if (request.startsWith(`${props.sessionID}:`)) pending.delete(request);
    }
    // A helper finishing cannot finish its parent; a helper's question still needs the human.
    if (!isRoot && !asked && !replied && !ended) return;
    if (isRoot) {
      if (busy || replied) rootStatus = "working";
      if (ended) rootStatus = "done";
    }
    const status = props.status ? { type: props.status.type } : undefined;
    const args = ["hook", "opencode", JSON.stringify({ type: event.type, properties: { status }, cwd: directory })];
    args.push("--status", pending.size ? "blocked" : rootStatus);
    await execute(executable, args, { timeout: 3000, windowsHide: true });
    warned = false;
  };
  return {
    event: async ({ event }) => {
      if (!process.env.SESHI_TERM_ID || !relevant.has(event.type)) return;
      // Preserve event order even when the host dispatches several callbacks together.
      queue = queue.then(() => send(event)).catch(async () => {
        if (!warned) {
          try {
            await client.app.log({ body: { service: "seshi", level: "warn", message: "Status report failed. Check that Seshi and its daemon are up to date." } });
          } catch {
            console.warn("Seshi could not report status or log the failure.");
          }
        }
        warned = true;
      });
      await queue;
    },
  };
};
