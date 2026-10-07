export interface OversightReview {
  review_id: string
  run_id: string
  status: string
  verdict: string | null
  model_verdict?: string | null
  circuit_node?: string | null
  human_dispositions?: Array<{ node_id: string; at_ms: number; action: string }>
  summary: string
  source_request_ids: string[]
  triggers: string[]
  finished_at: number | null
  actual_action_refs: string[]
  cancel_result?: { rollback_requested: boolean; restored_operations: number | null; error: string | null; files: Array<{ path: string; phase: string }> } | null
  proposals?: Array<{ proposal_id: string; state: string; reason: string; decision_source?: string; kind: string; result_refs: string[] }>
  work: { model: string; answers: string[]; notes: string[]; evidence: Array<{ entry_id: string; node_id: string | null; scope: string | null }> }
  usage?: Array<{ id: string; model: string; charged: number; state: string; cost_micros: number | null; currency: string }>
}
export interface OversightReports { run_id: string; reports: OversightReview[] }
