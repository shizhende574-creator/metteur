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

- Native Windows initially failed: a minimal program using installed Rust 1.99.0 returned `link.exe not found` (`native-linker-probe-7wf65372`). The owner subsequently requested installation. MSVC/SDK/CMake and LLVM are now installed and the functional tool probes pass; project-level native acceptance remains in progress.
- Cargo offline preflight could not resolve cached `anyhow`; no tests compiled (`native-shared-preflight-icmhm95q`).
- Local WSL Ubuntu 24.04 build prerequisites and exact Rust 1.99.0 are installed. The locked workspace build is in progress (`linux-workspace-build-l8jyjnig`); this is separate from native Windows validation.
- Paid model calls are permitted in principle, pending the owner's exact models, endpoints, data scope and spending limit. No real-model calls have been made.
- Mock browser tests do not replace real daemon/provider tests or cover all new NX surfaces. Backend integration, resource/task/schedule interactions, the 45-task matrix and final human visual review remain open.
- No remote publication, PR update or merge.

## Resource ownership and draft regressions

Additional directed probes found defects not covered by the former suite:

- A pending selection from workspace A survived A → B → A and repopulated cleared selections. Workspace epochs now synchronously invalidate old requests, including refreshes.
- Replacing the selection list during validation returned true for a new unchecked fragment. Validation now binds the exact items, and Chat also verifies the same snapshot IDs before sending.
- A late Project save acknowledgement closed a newer Personal edit. Save completion now binds the scope/workspace view and edit session.
- Filtering the resource list while editing sent the old body to the newly visible resource ID. The editor now retains its original resource/revision independently of filtering.
- Saving prevents duplicate submissions and preserves text typed after submission as a newer unsaved draft.

Initial failing evidence: `resource-races-before-fix-qtg1c3k4`, `resource-save-before-fix-tr0r3rt_`, and `resource-draft-binding-before-fix-kxnyoirb`. The first scope-save attempt also exposed an incorrect summary-button test locator; that locator was repaired before the actual draft-loss failure was recorded.

The repaired five resource cases plus affected Chat/composer/rewind/blueprint-save tests passed: **33 passed**, one worker, zero retries (`resource-chat-blueprint-regression-9ntxd8fk`). These are 28 reused regression cases and five new cases, not 33 additional distinct requirements.

This remains a mock gateway/browser check. Actual daemon ownership/permission checks and provider behavior remain separate acceptance layers.

## Further local verification work

- Marked the Blueprint custom node component raw and reused its type map. The four save-failure browser cases now also assert that rendering emits no reactive-component warning; all passed in the 33-case directed run.
- Pinned `verifyDepsBeforeRun: error`: scripts fail on stale dependencies instead of silently reinstalling them during other checks. An overlapping pnpm install previously caused an EPERM rename failure (`resource-race-types-eqv0gcd6`); this environment failure remains recorded. After explicit frozen installation, normal app/config checking passed (`current-frontend-types-xk9gxphl`) and lint passed (`current-frontend-lint-fhvpf6ch`).
- Replaced the NX fixture's `sleep 8` command with an actual Node timer and ensured its daemon inherits the launcher's Node directory in PATH. Native Windows shell execution and PATH handling passed four unit cases; identity checks passed three more (`fixture-identity-and-native-delay-l4urco9k`). This proves the command helper, not the complete native daemon flow.
- Navigation normalization/move/upgrade assertions passed (`navigation-order-jqkxezq4`); preview registry/production-entry isolation passed (`preview-isolation-e_qqnstl`). These checks are now included in the ordinary verification script.
- `LOCAL_ACCEPTANCE_MATRIX.json` enumerates all **45 NX tasks and 88 scenarios**, with unrun and partial local states retained. Model authorization is pending concrete configuration and budget; native Windows project validation is in progress.

## Project memory concurrency repairs

Three further directed browser probes reproduced source evidence appearing under a different note, filtering retargeting a draft to another note, and save acknowledgements discarding newer text (`memory-races-before-z4vx1zn1`: three semantic failures).

The memory editor now retains its original note and revision, invalidates source requests on selection/workspace changes, and binds save completion to its edit session. Newer text remains unsaved after the submitted revision is acknowledged. Workspace changes invalidate mutation completions synchronously.

All three probes passed after repair (`memory-races-fixed-666mybqp`), with effective app/config types (`memory-and-layout-types-m6bo3zjj`), lint (`memory-resource-lint-o1fcb7b9`) and production build (`frontend-production-memory-wszd5ubn`) passing. Native daemon memory validation remains separate.

The ordinary verification script also passed all seven steps (`default-gate/first-8umb2qrd`). A later direct production command detected a pnpm CI/non-CI virtual-store difference before building (`frontend-production-r2-mb78przy`); the workspace now explicitly selects the same local store layout in both modes. Typechecking and the latest production build passed in a normal shell without setting CI. Dependency versions did not change.

## Native Windows tools installed and exercised

The Microsoft-signed Build Tools installer installed Visual Studio Build Tools 2026 18.11.0, MSVC 14.51, Windows SDK 10.0.26100.0 and CMake 4.3.1 at `D:/Tocoding/VSBuildTools2026`. The official LLVM 23.1.3 x64 MSI was matched to the winget manifest SHA256 `5153f848ac87553118340f16f8d3cd4351d729bcba6e4785c6a4a9af29676de9` before installation at `D:/Tocoding/LLVM`. Both installers returned 0 and VS reports no reboot required. Installation logs are retained in the workspace review's `native-tools-install` directory.

Actual unprivileged probes passed (`native-tools-functional-f3p3rtvp`): Rust 1.99.0 linked and ran a native executable, MSVC compiled and ran a C++ executable using Windows SDK APIs, and Python loaded libclang.dll and queried its version. CMake and protoc executables were also exercised. Two earlier probe attempts failed in the test launcher's quoting/PATH resolution before compilation; both are retained (`native-installed-tool-probes-wzh3h0m1`, `native-installed-tool-probes-fixed-08y7yfqx`) and the launcher was corrected. These probes establish tool availability, not a project test pass.

The project's shared-module native Windows unit suite subsequently passed: **57 passed, 0 failed, 0 ignored** (`native-shared-tests-pse0g8zb`), with default test concurrency and a single compiler job. The separate daemon lock/storage/recovery cases are next; shared tests do not substitute for them.

The first native targeted-script attempt stopped before testing because Windows PowerShell 5 converted normal Cargo stderr progress into a terminating ErrorRecord (`native-daemon-targeted-5l3z16g9`). The script now captures native streams directly and checks the actual process exit code. A labelled native stub probe verifies both successful stderr handling and failure propagation: exit 7 stops later groups and produces no success marker (`windows-gate-contract-1ewoepf1`). This validates the script contract only. Actual daemon cases were restarted separately (`native-daemon-targeted-fixed-qm39jt7o`).

## Offline renderer worker exercised

Four native Windows worker checks passed (`native-offline-renderer-_3bhcros`): an exact-digest screenshot and console capture, blocked external fetch/image/frame and opaque-origin storage, rejection of a wrong digest, and rejection of an oversized snapshot. An owned loopback sentinel received **zero** requests. Chromium's sandbox stayed enabled; no user browser profile was used. The generated 1280×800 JPEG was inspected. This proves the shipped worker with synthetic snapshots; daemon adapter attachment/revocation and final user acceptance are separate.

## Blueprint launch review identity repaired

Two directed browser probes reproduced stale approval state when a proposal changed under the same request ID and an old response resolving a replacement request (`launch-review-before-qso4yc5d`). Review confirmation now binds the complete proposal, request and workspace; changes synchronously reset confirmation and invalidate old callbacks. Unmounting also invalidates pending responses.

Four directed cases passed (`launch-review-binding-g5fpfc5b`): changed proposal clears approval, old acknowledgement does not resolve a new request, policy changes require fresh approval and send the exact digest/scope/policy, and Decline records a negative decision without requiring approval. Effective types passed (`launch-review-types-iju5zaf8`), lint passed (`launch-review-lint-w4cf_zaz`) and the updated production build passed (`frontend-production-launch-8gde207w`). Existing EditorPane/chunk-size warnings remain. These are browser component fixtures; actual host version checks remain a separate layer.
