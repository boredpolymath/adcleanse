# Contributing to AdCleanse

Thank you for your interest in contributing to AdCleanse! We welcome contributions from privacy advocates, security researchers, and developers who share our commitment to user data sovereignty and ethical engineering.

## Zero-Telemetry & Security Ground Rules
Before submitting code, please ensure adherence to our core architectural constraints:
1. **Zero External Telemetry**: Do not introduce any third-party analytics, crash trackers, or telemetry beacons. Outbound network requests must only route to Meta endpoints or user-configured sync relays.
2. **Native Keyring for Secrets**: Never log, serialize, or store unencrypted session tokens in SQLite or localStorage. All secrets must go through the platform Keyring.
3. **Encrypted Local Storage**: All persistent records must reside in the SQLCipher-encrypted SQLite database.
4. **Memory Hygiene**: Use `zeroize` on transient buffers holding sensitive payloads.

## Development Workflow
1. Fork and clone the repository.
2. Review the comprehensive SDLC checklist in `/docs/adcleanse-master-sdlc-checklist.md`.
3. Verify formatting: `cargo fmt -- --check`.
4. Run compiler checks: `cargo clippy --all-targets --all-features -- -D warnings`.
5. Run test suite: `cargo test --all-targets`.

## License Agreement
By contributing to AdCleanse, you agree that your contributions will be licensed under its dual MIT and Apache-2.0 licenses.
