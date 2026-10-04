/** Show the exact server-bound plan change on both approval surfaces. */
export function blueprintApproval(payload: Record<string, unknown>) {
  const type = payload.request_type
  if (type !== 'replan_proposal' && type !== 'blueprint_save') return null
  const base = payload.base as Record<string, unknown> | undefined
  const file = typeof payload.path === 'string' ? payload.path : base?.blueprint_uri
  return {
    title: type === 'blueprint_save' ? 'Approve blueprint save' : 'Approve revised plan',
    detail: typeof payload.summary === 'string' ? payload.summary
      : `Review the proposed blueprint before saving${file ? ` to ${file}` : ''}.`,
    command: JSON.stringify({
      file, baseVersion: base ?? null, source: payload.source,
      affectedNodes: payload.affected_nodes, changes: payload.edits ?? payload.blueprint,
    }, null, 2),
  }
}
