# Codex app-server compatibility fixtures

These fixtures are hand-authored, sanitized examples, not captured sessions.
They contain no real config paths, accounts or transcripts.

- `codex-0.164.0.json`: required method/params references shared by verified
  Codex 0.164.0 and 0.165.0 schemas, combined in a test-only wrapper.
- `missing-interrupt.json`: the same inventory without `turn/interrupt`, to
  prove refusal before session mutation.
- `initialize-response.json`: a synthetic JSON-RPC response. It cannot
  establish method support.

See the [Codex app-server stack doc](../../../../docs/stack/codex-app-server.md)
for provenance and regeneration. The compatibility examples are not a full
JSON Schema validator or a replacement for the provider's generated definitions.
