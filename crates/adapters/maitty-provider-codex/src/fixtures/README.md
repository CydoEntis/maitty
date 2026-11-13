# Codex app-server compatibility fixtures

These fixtures are hand-authored, sanitized examples, not captured sessions.
They contain no real config paths, accounts or transcripts.

- `codex-0.164.0.json`: required method/params references shared by verified
  Codex 0.164.0 and 0.165.0 schemas, combined in a test-only wrapper.
- `codex-0.166.0.json`: required method/params references confirmed against
  the locally installed Codex 0.166.0 generated schemas.
- `missing-interrupt.json`: the same inventory without `turn/interrupt`, to
  prove refusal before session mutation.
- `initialize-response.json`: a synthetic JSON-RPC response. It cannot
  establish method support.

See the [Codex app-server stack doc](../../../../docs/stack/codex-app-server.md)
for provenance and regeneration. The compatibility examples are not a full
JSON Schema validator or a replacement for the provider's generated definitions.

- `codex-0.166.0.json` also lists `turn/steer`, which is optional: it is not in
  the 0.164.0 fixture, and `supports_turn_steer` reports it per connected Codex.
- `turn-steer-ack.json`: a synthetic `turn/steer` response (`TurnSteerResponse`
  has one required field, `turnId`).
