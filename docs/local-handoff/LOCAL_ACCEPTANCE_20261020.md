# Local acceptance in progress — 2026-10-20

Base: `ff7b795e536042e885f08a9bf7a2e5d4a3121749`. The owner requested local tests, comparison with the complete NX plan, repairs and quality improvements. Active branch: `codex/local-acceptance-20261020`. The original M1 checkout and downloaded package are preserved.

## Frontend gate repaired

- Reproduced all 33 effective application diagnostics. The former references-only root command did not establish application type safety.
- `pnpm typecheck` now checks the application and every root `*.config.ts`. Strict source checking stays enabled. Removed deprecated `baseUrl`; relative alias paths replace it. ES2023 library declarations cover the existing `findLast` calls without changing the ES2022 build target.
- Unified CodeMirror state/view at the already locked direct versions; frozen offline installation validated the targeted lockfile change. No general dependency upgrade.
- Repaired shared context/config types, message-role inference, reasoning defaults, Markdown renderer types, unused imports and an unreachable comparison. Concierge copying now runs in script and handles clipboard failure.
- Node checking skips third-party declarations, matching the existing Vue app configuration. This avoids unused optional PWA generator/worker declarations while checking all owned configs under `strict`; application diagnostics were repaired in source.
- Negative coverage probes inserted an invalid assignment separately in an app file and a config. The ordinary command rejected both with TS2322; probe files were removed.
- Fixed a mock Job defect: Kill could be followed by more output and a successful completion. A browser regression verifies one killed terminal notice, no later output and retained killed state.

Evidence parent: `D:/BaiduNetdiskDownload/metteur-a7-work/tmp/local-acceptance-20261020/`.

| Check | Result | Evidence directory |
|---|---|---|
| Baseline effective app types | Failed: 33 diagnostics | `baseline-app-types-omafq6xx` |
| Baseline Node config types | Failed: 17 third-party declaration diagnostics | `baseline-node-types-ihudqmrf` |
| Repaired app + config types | Passed | `repaired-types-18jq5b69` |
| Negative coverage probes | Both expected errors detected | `typecheck-coverage-directed-8wh7l5ua` |
| Lint | Passed | `fixed-web-lint-mvoav7dp` |
| Production frontend build | Passed; large-chunk warnings retained | `fixed-web-build-se3wyzo0` |
| Mock gateway browser suite | **94 passed**, 1 worker, 0 retries | `fixture-e2e-owned-port-5idnbprw` |

The first browser attempt failed before tests: Windows denied the chosen localhost/IPv6 port. The passing run bound IPv4 loopback and used an OS-assigned available port, without reusing another app's server. An initial coverage probe overlapped pnpm dependency recreation and failed as an environment error; `typecheck-coverage-bbznjlrb` is preserved and not counted as a coverage pass.

## Remaining acceptance

- Native Windows Rust is blocked: a minimal program using installed Rust 1.99.0 fails with `link.exe not found`. MSVC/SDK/CMake/LLVM setup is needed; the owner has been asked to install the administrator-level prerequisites. Evidence: `native-linker-probe-7wf65372`.
- Cargo offline preflight could not resolve cached `anyhow`; no tests compiled (`native-shared-preflight-icmhm95q`).
- Local WSL Ubuntu 24.04 is available. Linux build prerequisites are being prepared; Linux results will remain separate from native Windows results.
- Paid model calls are permitted in principle, pending the owner's exact models, endpoints, data scope and spending limit. No real-model calls have been made.
- Mock browser tests do not replace real daemon/provider tests or cover all new NX surfaces. Backend integration, resource/task/schedule interactions, the 45-task matrix and final human visual review remain open.
- No remote publication, PR update or merge.
