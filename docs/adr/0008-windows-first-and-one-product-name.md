# 0008. Windows is a first-class target, and the product name lives in one place

- **Status:** Accepted
- **Date:** 2026-11-06

## Context

Maitty ships on Windows, macOS and Linux. A change that only builds on Linux breaks a platform.
The product has been renamed twice, and a name copied into many files made each rename costly.

## Decision

- Every change builds and behaves on Windows (ConPTY, named pipes, PowerShell) as well as Linux
  and macOS. The cross-checks in the verification gate are not optional.
- The product name is set in one place (`crates/maitty-core/src/brand.rs` and `apps/tui/Cargo.toml`)
  and everything else derives from it.

## Alternatives considered

- **Treat Windows as best effort**: rejected, it is a stated user base.

## Consequences

- A rename is a change to one place plus the release assets.
