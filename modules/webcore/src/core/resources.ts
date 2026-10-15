export type ResourceMethod = 'queryProjectMemory' | 'writeProjectNote' | 'promoteProjectConvention' | 'updateProjectMemoryStatus'
export interface ResourceRef { scope: { scope: 'project' | 'personal'; owner: string }; id: string; revision: number; content_digest: string }
export interface MemoryEntry {
  reference: ResourceRef
  content: { title: string; body: string; origin: string; validity: string; author: string; source: string | null; files: { path: string; digest: string }[]; confirmation: unknown }
}
export interface MemoryPage { entries: MemoryEntry[]; next: string | null; truncated: boolean }
