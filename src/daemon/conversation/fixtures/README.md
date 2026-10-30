# Synthetic Codex protocol fixtures

These fixtures are hand-authored, sanitized examples, not generated schemas or
captured sessions. They contain no real config paths, accounts or transcripts.

- `codex-0.164.0.json`: the required method/params-reference subset shared by
  verified Codex 0.164.0 and 0.165.0 schemas, combined in a test-only wrapper.
- `missing-interrupt.json`: the same synthetic inventory with `turn/interrupt`
  deliberately absent, to prove refusal before session mutation.
- `initialize-response.json`: a synthetic JSON-RPC response with the four required
  initialize result fields. This response cannot establish method support.

Provenance and regeneration procedure: [provider stack doc](../../../../docs/stack/codex-app-server.md).
The installed `codex-cli 0.164.0` and `0.165.0` generated schemas were inspected
in isolated temporary folders; tests do not invoke Codex or read its home directory.
These subsets are compatibility examples, not a full JSON Schema validator or a
replacement for the provider's generated definitions.
