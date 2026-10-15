# Usability refinement R1 — staged preview

User request 2026-10-15 14:18:49 UTC: remove redundant small explanatory text from every new page; important functions at the top of the left rail; drag-adjustable persistent order; further experience-focused improvements.

Base: 9d6e4ad41370336d0dacfb3ea089f9b68a41114c. Original source and bundle preserved. No production integration yet; no backend/model calls in the isolated preview.

## Scope
- Default rail: Chat, Explorer, Execution, Task continuation, Project memory, Prompt resources, Schedules, Version Flow, Addon pages. Settings remains pinned.
- Real shared rail component: pointer drag, before/after indicators, local persistence, reset order; Alt+Up/Down and an accessible menu also support keyboard/touch. Unknown/duplicate/removed saved IDs cannot remove navigation. New items append safely.
- Remove duplicated uppercase eyebrows and always-visible teaching paragraphs from all new surfaces. Purpose/help moves to keyboard-focusable tooltips. Keep field values, status, source records, errors, exact authority/budget/irreversible-action confirmations visible where needed.
- Memory: compact heading/search, readable note body, source/digest/file metadata under Source details. Keep current vs verified distinction available from status help; do not hide a stale-version warning.
- Prompts: aligned Project/Personal and search toolbar; clearer Use in Chat action (select, open composer, no automatic send); reduce empty drop-target scaffolding. Keep actual selections and source revision checks, not just the visuals.
- Tasks/schedules: consistent compact headers and action positions; readable budget/wait state; technical details on demand. Avoid dead-end actions without an eligible task. Keep Stop consequences and plan confirmation explicit.
- Blueprint run review: remove decorative tagline; retain concrete scope, dependency versions, source permission, completion policy and final confirmation.
- Addon inspector: remove duplicate page eyebrows/footer prose; scoped explanation on demand; retain evidence states, target revision and explicit independent human acceptance.

## Evidence so far
Vue typecheck and isolated build passed. Ten order-helper assertions passed. Actual cloud-browser Alt+Up move survives reload; actual native pointer drag moved Addon pages from last to second; reset restored defaults. Preview storage key is separate from the production preference.

This preview's resource/task records are synthetic. Non-navigation action buttons demonstrate intended placement and do not execute RPCs. Permission and budget dialogs retain production semantics. Next: present concrete screenshots, obtain integration confirmation, then connect approved refinements to existing production data and perform only necessary UI checks.
