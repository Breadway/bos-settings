# BOS Settings v2 test builds

Main gap implementation commit: `19cbca07101fa7612f17f8e2600d5254ddf8a65d` on `feature/bos-settings-v2`.

The final build also includes the equivalent package-query success expressions required by the CI runner's Rust 1.96 Clippy. Generated Lua tests use a vendored test-only interpreter, so CI does not require a system Lua executable.

## Settings

- Executable: `/home/breadway/Projects/bos-settings/src/target/release/bos-settings`
- SHA256: `5236ba7ee921cad25471b4cb161d129a04bdeb2809fc0d0552211c5225559eba`
- Production frontend built before the Rust release binary.
- All 23 generated JavaScript, CSS, JSON, font and license assets matched bytes embedded in the executable after Brotli decompression.
- Repeat from the repository root: `node scripts/verify-embedded-frontend.mjs`.
- Frontend check: zero errors and warnings. Frontend tests and production build pass.
- Rust fmt and clippy with warnings denied pass; 134 tests pass; `cargo build --release --locked` succeeds.

## Companion bar

- Executable: `/tmp/bos-settings-v2-test/breadbar`
- SHA256: `76e2ca2b41f23101ff4ab28dedeaf83d1f4bb53bb2f1bb837e753a816dcfa619`
- Commit: `8263a8f11788fe53282cd45c0529a1264884d418` on `feature/notification-preferences`.
- PR: https://git.breadway.dev/Breadway/breadbar/pulls/57
- Test build uses documented dev dependency overrides, with debug information removed from the copied binary. Committed dependency pins and Cargo.lock remain unchanged.
- Clippy with warnings denied and 24 headless notification tests pass. Forgejo CI passed at this commit.

The companion bar must be running to test the new notification preference API, clock-format and bar text integration. Neither executable was launched during this implementation session. No live configuration writes or service starts were used for testing. Rendered pixel accuracy and hardware interactions remain for the user's UI test.
