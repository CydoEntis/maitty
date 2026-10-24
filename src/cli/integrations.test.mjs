import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { test } from "node:test";
import { promisify } from "node:util";
import { SourceTextModule, SyntheticModule, createContext } from "node:vm";

async function adapter(name, inside = true, argv = ["node", "opencode", "run"], extraEnv = {}) {
  const reports = [];
  let fail = false;
  const polls = new Set();
  const timers = new Map();
  const context = createContext({
    process: { env: inside ? { VIBECASSO_TERM_ID: "7", ...extraEnv } : {}, argv }, console,
    setTimeout: name === "omp" ? (callback, delay) => { const timer = { unref() {} }; timers.set(timer, { callback, delay }); return timer; } : setTimeout,
    clearTimeout: name === "omp" ? (timer) => timers.delete(timer) : clearTimeout,
    setInterval: (callback) => { polls.add(callback); return callback; },
    clearInterval: (callback) => polls.delete(callback),
    AbortSignal, AbortController,
  });
  const child = new SyntheticModule(["execFile"], function () {
    this.setExport("execFile", (exe, args, options, done) => {
      assert.equal(exe, "C:\\Program Files\\vibecasso\\vibecasso.exe");
      assert.equal(options.windowsHide, true);
      assert.ok(options.timeout <= 3000);
      const payload = JSON.parse(args[2]);
      const status = args[3] === "--status" ? args[4] : undefined;
      reports.push({ agent: args[1], event: payload.hook_event_name ?? payload.type, status, payload });
      if (fail) done(new Error("test daemon unavailable"));
      else done(null, "", "");
    });
  }, { context });
  const util = new SyntheticModule(["promisify"], function () { this.setExport("promisify", promisify); }, { context });
  const source = (await readFile(new URL(`${name}.js`, import.meta.url), "utf8")).replace("__VIBECASSO_EXE__", JSON.stringify("C:\\Program Files\\vibecasso\\vibecasso.exe"));
  const module = new SourceTextModule(source, { context });
  await module.link((id) => {
    if (id === "node:child_process") return child;
    if (id === "node:util") return util;
    throw new Error(`unexpected dependency ${id}`);
  });
  await module.evaluate();
  return { api: module.namespace, reports,
    poll() { for (const fn of polls) fn(); },
    fireTimers(ms) { for (const [timer, item] of timers) if (item.delay <= ms) { timers.delete(timer); item.callback(); } },
    setFail(value) { fail = value; } };
}

test("Pi follows native lifecycle, nested dialogs, settlement and recovery without model assumptions", async () => {
  const { api, reports, setFail } = await adapter("pi");
  const handlers = new Map();
  const notices = [];
  const ctx = { cwd: "/project", hasUI: true, ui: { notify: (...args) => notices.push(args) } };
  api.default({ on: (event, handler) => handlers.set(event, handler) });
  const emit = (event) => handlers.get(event)({}, ctx);
  await emit("session_start");
  await emit("agent_start");
  await emit("ui_prompt_start");
  await emit("ui_prompt_start");
  await emit("ui_prompt_end");
  await emit("ui_prompt_end");
  assert.ok(!handlers.has("agent_end"), "an intermediate end cannot finish retries/compaction");
  await emit("agent_settled");
  await emit("ui_prompt_start");
  await emit("ui_prompt_end");
  await emit("session_shutdown");
  assert.deepEqual(reports.map((r) => r.status ?? r.event), ["idle", "working", "pi.ui_prompt_start", "pi.ui_prompt_start", "blocked", "working", "done", "pi.ui_prompt_start", "idle", "pi.session_shutdown"]);
  setFail(true);
  await emit("agent_start");
  await emit("agent_start");
  assert.equal(notices.length, 1, "report failures surface once without breaking the agent");
  setFail(false);
  await emit("agent_settled");
  assert.equal(reports.at(-1).status, "done");
});

test("OpenCode preserves pending questions across busy events and helper completion", async () => {
  const { api, reports } = await adapter("opencode");
  const client = { session: { get: async ({ sessionID }) => ({ data: { id: sessionID, parentID: sessionID === "child" ? "root" : undefined } }) }, app: { log: async () => {} } };
  const plugin = await api.Vibecasso({ directory: "/project", client });
  const emit = (type, properties) => plugin.event({ event: { type, properties } });
  await emit("session.status", { sessionID: "root", status: { type: "busy", message: "not forwarded" } });
  await Promise.all([
    emit("question.asked", { sessionID: "child", id: "q1" }),
    emit("permission.asked", { sessionID: "root", id: "p1" }),
    emit("session.status", { sessionID: "root", status: { type: "busy" } }),
  ]);
  await emit("permission.replied", { sessionID: "root", requestID: "p1" });
  await emit("question.replied", { sessionID: "child", requestID: "q1" });
  await emit("session.idle", { sessionID: "child" });
  assert.equal(reports.at(-1).status, "working", "helper completion does not finish parent");
  await emit("session.idle", { sessionID: "root" });
  await emit("session.idle", { sessionID: "unrelated" });
  await emit("message.updated", { sessionID: "root", text: "not forwarded" });
  assert.deepEqual(reports.map((r) => r.status), ["working", "blocked", "blocked", "blocked", "blocked", "working", "working", "done"]);
  assert.ok(reports.every((r) => !JSON.stringify(r.payload).includes("not forwarded")));
});

test("OpenCode recovers after a daemon error and remains inert outside Vibecasso", async () => {
  const { api, reports, setFail } = await adapter("opencode");
  const logs = [];
  const client = { session: { get: async () => ({ data: { id: "root" } }) }, app: { log: async (entry) => logs.push(entry) } };
  const plugin = await api.Vibecasso({ directory: "/project", client });
  setFail(true);
  await plugin.event({ event: { type: "session.status", properties: { sessionID: "root", status: { type: "busy" } } } });
  setFail(false);
  await plugin.event({ event: { type: "session.error", properties: { sessionID: "root" } } });
  assert.equal(logs.length, 1);
  assert.equal(reports.at(-1).status, "done");
  const outside = await adapter("opencode", false);
  await (await outside.api.Vibecasso({ directory: "/project", client })).event({ event: { type: "session.idle", properties: { sessionID: "root" } } });
  assert.equal(outside.reports.length, 0);
  const pi = await adapter("pi", false);
  pi.api.default({ on: () => assert.fail("must not subscribe outside Vibecasso") });
});

test("Pi reload resumes work, RPC does not report and false settlement cannot finish it", async () => {
  const { api, reports } = await adapter("pi");
  const handlers = new Map();
  api.default({ on: (name, handler) => handlers.set(name, handler) });
  const ctx = { mode: "tui", isIdle: () => false, cwd: "/project" };
  await handlers.get("session_start")({}, ctx);
  await handlers.get("agent_settled")({}, ctx);
  assert.deepEqual(reports.map((r) => r.status), ["working"]);
  await handlers.get("session_start")({}, { ...ctx, mode: "rpc", hasUI: true });
  assert.equal(reports.length, 1);
});

test("OpenCode shared server and interactive TUI leave reporting to the selected-session bridge", async () => {
  for (const argv of [["opencode"], ["opencode", "serve"], ["opencode", "web"], ["opencode", "attach"], ["opencode", "run", "--attach", "http://localhost"]]) {
    const { api } = await adapter("opencode", true, ["node", ...argv]);
    assert.equal((await api.Vibecasso({ directory: "/project", client: {} })).event, undefined, argv.join(" "));
  }
});

async function flush() {
  for (let i = 0; i < 12; i++) await new Promise(setImmediate);
}

test("OpenCode TUI v1 scopes shared-backend blockers, helper completion and route changes", async () => {
  const bridge = await adapter("opencode-tui");
  const sessions = new Map([["root", { id: "root" }], ["child", { id: "child", parentID: "root" }], ["other", { id: "other" }]]);
  const statuses = { root: { type: "busy" }, other: { type: "busy" } };
  const listeners = new Map();
  let dispose;
  const route = { current: { name: "session", params: { sessionID: "root" } } };
  const api = {
    route, state: { session: { get: (id) => sessions.get(id) } },
    client: {
      session: { get: async ({ sessionID }) => ({ data: sessions.get(sessionID) }), status: async () => ({ data: statuses }) },
      permission: { list: async () => ({ data: [] }) }, question: { list: async () => ({ data: [] }) },
    },
    event: { on(type, callback) { listeners.set(type, callback); return () => listeners.delete(type); } },
    lifecycle: { onDispose(callback) { dispose = callback; } },
  };
  const emit = async (type, properties) => {
    listeners.get(type)?.({ type, properties });
    await flush();
  };
  try {
    await bridge.api.default.tui(api);
    await flush();
    assert.equal(bridge.reports.at(-1).status, "working");
    await emit("session.status", { sessionID: "child", status: { type: "busy" } });
    assert.ok(bridge.reports.some((report) => report.event === "SubagentStart" && report.payload.agent_id === "child"), "reports the active child under its selected parent");
    await emit("session.idle", { sessionID: "child" });
    assert.ok(bridge.reports.some((report) => report.event === "SubagentStop" && report.payload.agent_id === "child"), "removes the child when it finishes");
    await emit("question.asked", { sessionID: "other", id: "unrelated" });
    assert.ok(bridge.reports.some((report) => report.event === "opencode.tui" && report.status === "working"), "the parent remains working while another session asks a question");
    await emit("question.asked", { sessionID: "child", id: "q1" });
    assert.equal(bridge.reports.at(-1).status, "blocked");
    await emit("session.idle", { sessionID: "child" });
    assert.equal(bridge.reports.at(-1).status, "blocked");
    await emit("question.replied", { sessionID: "child", requestID: "q1" });
    assert.equal(bridge.reports.at(-1).status, "working");
    await emit("session.idle", { sessionID: "root" });
    assert.equal(bridge.reports.at(-1).status, "done");
    route.current = { name: "session", params: { sessionID: "other" } };
    bridge.poll();
    await flush();
    assert.equal(bridge.reports.at(-1).payload.session_id, "other");
    assert.equal(bridge.reports.at(-1).status, "working");
  } finally { dispose?.(); }
  const count = bridge.reports.length;
  bridge.poll();
  await flush();
  assert.equal(bridge.reports.length, count);
});

test("OpenCode TUI v2 never attributes another pane's events or emits fake initial completion", async () => {
  const bridge = await adapter("opencode-tui");
  const sessions = new Map([["root", { id: "root" }], ["child", { id: "child", parentID: "root" }], ["other", { id: "other" }]]);
  const sessionStatuses = new Map([["root", "idle"], ["child", "idle"], ["other", "idle"]]);
  let route = { type: "session", sessionID: "root" };
  let receive;
  const api = {
    ui: { router: { current: () => route } },
    data: {
      session: {
        get: (id) => sessions.get(id), family: (id) => id === "root" ? ["child"] : [],
        status: (id) => sessionStatuses.get(id) ?? "idle", permission: { list: () => [] }, form: { list: () => [] },
      },
      listen: (callback) => { receive = callback; return () => { receive = undefined; }; },
    },
  };
  const emit = async (type, data) => { receive?.({ details: { type, data } }); await flush(); };
  const dispose = bridge.api.default.setup(api);
  try {
    await flush();
    assert.ok(!bridge.reports.some((r) => r.status === "done"));
    await emit("session.execution.started", { sessionID: "root" });
    assert.equal(bridge.reports.at(-1).status, "working");
    sessionStatuses.set("child", "running");
    await emit("session.execution.started", { sessionID: "child" });
    assert.ok(bridge.reports.some((report) => report.event === "SubagentStart" && report.payload.agent_id === "child"), `reports a running descendant: ${JSON.stringify(bridge.reports)}`);
    sessionStatuses.set("child", "idle");
    await emit("session.execution.succeeded", { sessionID: "child" });
    assert.ok(bridge.reports.some((report) => report.event === "SubagentStop" && report.payload.agent_id === "child"), "clears the descendant when it settles");
    await emit("form.created", { form: { sessionID: "other", id: "unrelated" } });
    assert.equal(bridge.reports.at(-1).status, "working");
    await emit("form.created", { form: { sessionID: "child", id: "q1" } });
    assert.equal(bridge.reports.at(-1).status, "blocked");
    await emit("session.execution.succeeded", { sessionID: "child" });
    assert.equal(bridge.reports.at(-1).status, "blocked");
    await emit("form.replied", { sessionID: "child", id: "q1" });
    assert.equal(bridge.reports.at(-1).status, "working");
    await emit("session.execution.succeeded", { sessionID: "root" });
    assert.equal(bridge.reports.at(-1).status, "done");
    route = { type: "session", sessionID: "other" };
    bridge.poll();
    await flush();
    assert.equal(bridge.reports.at(-1).payload.session_id, "other");
    assert.equal(bridge.reports.at(-1).status, "idle");
  } finally { dispose(); }
  assert.equal(receive, undefined);
});

test("OMP keeps continuations and retry holds working and counts overlapping native prompts", async () => {
  const bridge = await adapter("omp");
  const handlers = new Map();
  const ctx = { hasUI: true, isIdle: () => false, cwd: "/project" };
  bridge.api.default({ on: (event, handler) => handlers.set(event, handler), events: { on() {} } });
  const emit = async (event, data = {}) => { await handlers.get(event)(data, ctx); await flush(); };
  await emit("session_start");
  assert.equal(bridge.reports.at(-1).status, "working");
  await emit("tool_execution_start", { toolName: "ask" });
  await emit("tool_approval_requested");
  await emit("tool_execution_end", { toolName: "ask" });
  assert.equal(bridge.reports.at(-1).status, "blocked");
  await emit("tool_approval_resolved");
  assert.equal(bridge.reports.at(-1).status, "working");
  await emit("agent_end", { willContinue: true });
  bridge.fireTimers(3000); await flush();
  assert.equal(bridge.reports.at(-1).status, "working");
  await emit("agent_end", { messages: [{ role: "assistant", stopReason: "error", errorMessage: "429 rate limit" }] });
  await emit("agent_end");
  bridge.fireTimers(250); await flush();
  assert.equal(bridge.reports.at(-1).status, "working");
  bridge.fireTimers(2500); await flush();
  assert.equal(bridge.reports.at(-1).status, "blocked");
  await emit("agent_start");
  await emit("agent_end");
  bridge.fireTimers(250); await flush();
  assert.equal(bridge.reports.at(-1).status, "done");
  await emit("session_shutdown");
  const nested = await adapter("omp", true, undefined, { OMPCODE: "1" });
  nested.api.default({ on() { assert.fail("nested OMP cannot own the pane"); } });
});

test("Kilo keeps needs-you until every native request resolves", async () => {
  const bridge = await adapter("kilo");
  const plugin = await bridge.api.Vibecasso();
  await plugin["chat.message"]({ sessionID: "root" });
  const emit = (type, properties) => plugin.event({ event: { type, properties: { sessionID: "root", ...properties } } });
  await emit("permission.asked", { id: "p1" });
  await emit("question.asked", { id: "q1" });
  await emit("session.status", { status: "busy" });
  await emit("permission.replied", { requestID: "p1" });
  assert.equal(bridge.reports.at(-1).status, "blocked");
  await emit("question.replied", { requestID: "q1" });
  assert.equal(bridge.reports.at(-1).status, "working");
  await emit("session.idle");
  assert.equal(bridge.reports.at(-1).status, "done");
  const outside = await adapter("kilo", false);
  assert.equal((await outside.api.Vibecasso()).event, undefined);
});

test("Kilo retries later events when both reporting and its native log fail", async () => {
  const bridge = await adapter("kilo");
  const plugin = await bridge.api.Vibecasso({ client: { app: { log: async () => { throw new Error("log unavailable"); } } } });
  bridge.setFail(true);
  await plugin["chat.message"]({ sessionID: "root" });
  bridge.setFail(false);
  await plugin.event({ event: { type: "session.idle", properties: { sessionID: "root" } } });
  assert.equal(bridge.reports.at(-1).status, "done");
});
