import { defineStore } from 'pinia'
import { reactive } from 'vue'
import { gateway } from '@/core'
import { readDocument, writeDocument, parsePromptFile } from '@/lib/prompt-file'
import type { PromptEntry, ResourceRef } from '@/core/resources'
import { useRecoveryStore } from './recovery'
import { hash } from 'ohash'

export interface EditorDocument {
  workspace: string
  path: string
  content: string
  baseline: string
  disk: string
  loaded: boolean
  deleted: boolean
  conflict: boolean
  revision: number
  saving: boolean
  error: string
  resource?: PromptEntry
  diskResource?: PromptEntry
}

/** Buffers belong to workspace documents, never to a mounted editor component. */
export const useDocumentsStore = defineStore('documents', () => {
  const recovery = useRecoveryStore()
  const recoveryKind = (path: string) =>
    path.endsWith('.blueprint') ? ('canvas' as const) : ('text' as const)
  const records = reactive(new Map<string, EditorDocument>())
  const reads = new Map<string, Promise<EditorDocument>>()
  const pendingSync = new Set<EditorDocument>()
  const syncTickets = new WeakMap<EditorDocument, number>()
  const invalidateSync = (doc: EditorDocument) => {
    const ticket = (syncTickets.get(doc) ?? 0) + 1
    syncTickets.set(doc, ticket)
    return ticket
  }
  const key = (workspace: string, path: string) => JSON.stringify([workspace, path])
  const peek = (workspace: string, path: string) => records.get(key(workspace, path))
  function get(workspace: string, path: string) {
    const id = key(workspace, path)
    if (!records.has(id))
      records.set(id, {
        workspace,
        path,
        content: '',
        baseline: '',
        disk: '',
        loaded: false,
        deleted: false,
        conflict: false,
        revision: 0,
        saving: false,
        error: '',
      })
    return records.get(id)!
  }
  const dirty = (doc: EditorDocument) => doc.deleted || doc.content !== doc.baseline
  function edit(workspace: string, path: string, content: string) {
    const doc = get(workspace, path)
    if (doc.content !== content) {
      doc.content = content
      doc.revision++
      recovery.edit(workspace, recoveryKind(path), path, {
        raw: content,
        baseline_identity: `xxh3:${hash(doc.baseline)}`,
        view: { baseline_content: doc.baseline },
      })
    }
  }
  async function open(workspace: string, path: string) {
    const doc = get(workspace, path),
      id = key(workspace, path)
    if (doc.loaded) {
      void sync(workspace, path)
      return doc
    }
    if (reads.has(id)) return reads.get(id)!
    const pending = (async () => {
      const revision = doc.revision
      try {
        const result = await readDocument(workspace, path)
        if (!result.ok) {
          doc.error = result.error
          return doc
        }
        if (revision === doc.revision) doc.content = doc.baseline = doc.disk = result.data.content
        doc.loaded = true
        doc.error = ''
        doc.resource = result.data.resource
        doc.diskResource = result.data.resource
        return doc
      } finally {
        reads.delete(id)
      }
    })()
    reads.set(id, pending)
    return pending
  }
  async function sync(workspace: string, path: string) {
    const doc = get(workspace, path)
    if (!doc.loaded) return
    if (doc.saving) {
      pendingSync.add(doc)
      return
    }
    const revision = doc.revision
    const ticket = invalidateSync(doc)
    const current = () =>
      ticket === syncTickets.get(doc) && revision === doc.revision && !doc.saving
    const result = await readDocument(workspace, path)
    if (!current()) return
    if (!result.ok) {
      // Only a confirmed missing file is deletion; transport/permission errors preserve buffers.
      if (parsePromptFile(path)) {
        if (/not.found|missing|deleted/i.test(result.error)) doc.deleted = true
        return
      }
      const stat = await gateway.statFile(workspace, path)
      if (!current()) return
      if (!stat.ok && /not.found|no such file|cannot find|os error 2/i.test(stat.error))
        doc.deleted = true
      return
    }
    const disk = result.data.content
    doc.deleted = false
    doc.disk = disk
    doc.diskResource = result.data.resource
    if (dirty(doc))
      doc.conflict =
        (disk !== doc.baseline && disk !== doc.content) ||
        (doc.resource !== undefined &&
          (doc.resource.reference.revision !== result.data.resource?.reference.revision ||
            doc.resource.reference.content_digest !==
              result.data.resource?.reference.content_digest))
    else {
      doc.content = doc.baseline = disk
      doc.conflict = false
      doc.resource = result.data.resource
    }
  }
  async function save(workspace: string, path: string, overwrite = false) {
    const doc = get(workspace, path)
    if (doc.saving) return false
    if (doc.conflict && !doc.deleted && !overwrite) {
      doc.error =
        'File content changed on disk; review the current version before saving this draft.'
      return false
    }
    const content = doc.content
    invalidateSync(doc)
    doc.saving = true
    try {
      const result = await writeDocument(
        workspace,
        path,
        content,
        doc.resource,
        doc.deleted
          ? { createOnly: true }
          : overwrite
            ? undefined
            : { expectedContent: doc.baseline },
      )
      if (!result.ok) {
        doc.error = result.error
        return false
      }
      // Edits made while the save was in flight stay dirty against the saved snapshot.
      doc.baseline = doc.disk = content
      doc.deleted = false
      doc.conflict = false
      doc.error = ''
      if (result.data) doc.resource = doc.diskResource = result.data
      // Cleanup is conditional on the exact saved content. A newer edit or
      // an unknown backup receipt keeps its recovery copy intact.
      void recovery.saved(workspace, recoveryKind(path), path, content)
      return true
    } finally {
      doc.saving = false
      if (pendingSync.delete(doc) && records.get(key(workspace, path)) === doc)
        void sync(workspace, path)
    }
  }
  function markDeleted(workspace: string, path: string) {
    for (const doc of records.values()) {
      if (doc.workspace === workspace && (doc.path === path || doc.path.startsWith(path + '/'))) {
        invalidateSync(doc)
        doc.deleted = true
      }
    }
  }
  function discard(workspace: string, path: string) {
    const doc = get(workspace, path)
    invalidateSync(doc)
    doc.content = doc.baseline = doc.disk
    doc.conflict = false
    doc.resource = doc.diskResource
    doc.revision++
  }
  function adoptReviewedResource(workspace: string, path: string, reviewed: ResourceRef) {
    const doc = get(workspace, path),
      latest = doc.diskResource
    if (
      !latest ||
      latest.reference.id !== reviewed.id ||
      latest.reference.revision !== reviewed.revision ||
      latest.reference.content_digest !== reviewed.content_digest
    )
      return false
    invalidateSync(doc)
    doc.resource = latest
    doc.baseline = doc.disk
    doc.conflict = false
    doc.error = ''
    return true
  }
  function forget(workspace: string, path: string) {
    records.delete(key(workspace, path))
  }
  function rename(workspace: string, from: string, to: string) {
    recovery.renamed(workspace, from, to)
    for (const [id, doc] of [...records]) {
      if (doc.workspace !== workspace || (doc.path !== from && !doc.path.startsWith(from + '/')))
        continue
      records.delete(id)
      doc.path = to + doc.path.slice(from.length)
      records.set(key(workspace, doc.path), doc)
    }
  }
  return {
    records,
    get,
    peek,
    open,
    edit,
    sync,
    save,
    dirty,
    markDeleted,
    discard,
    adoptReviewedResource,
    forget,
    rename,
    recoveryKind,
  }
})
