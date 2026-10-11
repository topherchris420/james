## Purpose

What concrete problem does this change solve?

## Behavior and evidence

Describe actual behavior and failure-path tests. Distinguish code-level checks from host/platform results.

## Verification

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo test --workspace --all-targets --locked --offline`
- [ ] `cargo clippy --workspace --all-targets --locked --offline -- -D warnings`
- [ ] Additional checks and environment listed below

## Security, privacy and limits

Describe untrusted input handling, scopes, credentials, cancellation, unresolved effects and retention. Mark not applicable with justification.

## Documentation and provenance

- [ ] Docs describe implemented behavior, not plans as features
- [ ] Added code/assets have known licensing and origin
- [ ] Untested platforms and known limitations are explicit

## Rollback

Explain rollback and any persistent-state migration considerations.
