# Preview inspector example

This self-contained page uses only the scoped MessageChannel bridge. It cannot call the daemon directly, decide permissions, run a shell, open arbitrary URLs or query personal resources.

The package must be signed with the operator's trusted signing workflow before production installation. No production private signing key is included. The targeted test generates a clearly designated self-signed fixture, only for the isolated local demonstration.

Register up to 32 local HTML previews in the workspace's `.metteur/preview-targets.json`, for example:

```json
[{"id":"home","title":"Homepage","revision":1,"file":"previews/home.html"}]
```

Each file must be an existing workspace-relative HTML file outside `.metteur`, at most 256 KiB and valid UTF-8. No URL, traversal or outside-workspace symlink is admitted. The host selector chooses a target; the page cannot select or switch targets itself. Changes to either revision or content invalidate a pending review.

Checks currently cover bounded source validity, doctype, title and viewport declarations. These are source diagnostics, not claims of visual correctness, accessibility compliance or browser-console capture. Browser evidence and model opinion remain explicitly absent until a corresponding supported checker is available. No model calls are made.

Reports and independent human review events are saved as append-only host-owned JSON records under the daemon's addon-state/inspection-reports directory. Human acceptance never writes a sandbox grant.
