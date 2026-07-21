<script setup lang="ts">
import type { Editor } from '@tiptap/vue-3'
import type { Page } from '~/stores/pages'

definePageMeta({ middleware: ['auth'] })

const authStore = useAuthStore()
const pagesStore = usePagesStore()
const workspacesStore = useWorkspacesStore()
const route = useRoute()

const shareOpen = ref(false)
const historyOpen = ref(false)
// Bumped after a version restore so <Editor> remounts with a fresh Y.Doc that
// re-syncs from the restored page_content (and a new collab room on the server).
const editorEpoch = ref(0)
const commentsOpen = ref(false)
const focusedCommentMarkId = ref<string | null>(null)

function onVersionRestored() {
  editorEpoch.value += 1
  // Refresh page metadata (updated_at) so the topbar "Edited …" updates.
  if (authStore.workspace) pagesStore.fetchPages(authStore.workspace.id)
}
const createWorkspaceOpen = ref(false)
const membersOpen = ref(false)
const trashOpen = ref(false)

function openComments(markId?: string) {
  focusedCommentMarkId.value = markId ?? null
  commentsOpen.value = true
}

// TipTap instance for the open page — used by the right-side table of contents.
const editorInstance = ref<Editor | null>(null)
const editorScrollRoot = ref<HTMLElement | null>(null)

// A page shared with this user (via link or a direct grant) that isn't in
// their own workspace's page list — including an anonymous visitor with no
// workspace/account at all, following a public link.
const sharedPage = ref<Page | null>(null)
const sharedPageLoading = ref(false)
const sharedPageRateLimited = ref(false)
const linkToken = computed(() => {
  const q = route.query.link
  return typeof q === 'string' ? q : null
})

const activePage = computed(
  () => pagesStore.pages.find((p) => p.id === pagesStore.activePageId) ?? sharedPage.value,
)

// Who else is currently viewing/editing the open page, reported live by Editor's
// awareness subscription. Reset on page switch so a stale list doesn't flash.
const presentUsers = ref<
  { clientId: number; name: string; email: string | null; color: string; avatarUrl: string | null }[]
>([])
watch(
  () => activePage.value?.id,
  () => {
    presentUsers.value = []
    editorInstance.value = null
  },
)

const { record: recordRecent } = useRecents()

async function loadSharedPage(id: string) {
  sharedPage.value = null
  sharedPageRateLimited.value = false
  sharedPageLoading.value = true
  try {
    sharedPage.value = await pagesStore.fetchPage(id, linkToken.value)
  } catch (err: any) {
    sharedPage.value = null
    sharedPageRateLimited.value = err?.response?.status === 429
  } finally {
    sharedPageLoading.value = false
  }
}

watch(
  () => pagesStore.activePageId,
  (id) => {
    if (!id || pagesStore.pages.some((p) => p.id === id)) {
      sharedPage.value = null
      return
    }
    loadSharedPage(id)
  },
  { immediate: true },
)

watch(
  activePage,
  (page) => {
    if (page) recordRecent({ id: page.id, title: page.title, icon: page.icon })
  },
  { immediate: true },
)

// URL is the source of truth for which page is open, so refresh/back/forward
// land on the same document. pagesStore.activePageId mirrors it both ways:
// route -> store keeps direct link/refresh/navigation working, store ->
// route pushes a URL whenever a page is selected elsewhere (e.g. PageTree).
watch(
  () => route.params.pageId,
  (id) => {
    pagesStore.activePageId = (Array.isArray(id) ? id[0] : id) || null
  },
  { immediate: true },
)

watch(
  () => pagesStore.activePageId,
  (id) => {
    const routeId = route.params.pageId
    const currentId = Array.isArray(routeId) ? routeId[0] : routeId
    if ((id || null) !== (currentId || null)) {
      navigateTo(id ? `/app/${id}` : '/app')
    }
  },
)

onMounted(async () => {
  if (!authStore.workspace) return
  await pagesStore.fetchPages(authStore.workspace.id)
  await workspacesStore.fetchAll()
  const savedId = localStorage.getItem('activeWorkspaceId')
  if (savedId && savedId !== authStore.workspace.id) {
    const match = workspacesStore.list.find((ws) => ws.id === savedId)
    if (match) workspacesStore.setActive(match)
  }
})

watch(
  () => authStore.workspace?.id,
  (id, old) => {
    if (id && old && id !== old) {
      pagesStore.activePageId = null
      pagesStore.fetchPages(id)
    }
  },
)
</script>

<template>
  <div
    v-if="!authStore.workspace && !activePage"
    class="min-h-screen bg-white p-8 font-sans text-neutral-600 dark:bg-neutral-950 dark:text-neutral-400"
  >
    <template v-if="sharedPageLoading">Loading…</template>
    <template v-else-if="sharedPageRateLimited">
      Too many requests right now — please wait a moment.
      <button
        type="button"
        class="ml-2 underline hover:text-neutral-900 dark:hover:text-neutral-100"
        @click="pagesStore.activePageId && loadSharedPage(pagesStore.activePageId)"
      >
        Retry
      </button>
    </template>
    <template v-else-if="linkToken">This link is invalid, expired, or you don't have access.</template>
    <template v-else>No workspace yet.</template>
  </div>

  <div v-else class="flex h-screen font-sans">
    <AppSidebar
      v-if="authStore.workspace"
      :workspace-id="authStore.workspace.id"
      @open-create="createWorkspaceOpen = true"
      @open-members="membersOpen = true"
      @open-trash="trashOpen = true"
    />

    <div class="flex min-w-0 flex-1 flex-col bg-white dark:bg-neutral-900">
      <AppTopbar
        :active-page="activePage"
        :present-users="presentUsers"
        @open-share="shareOpen = true"
        @open-history="activePage?.role !== 'viewer' && (historyOpen = true)"
        @open-comments="openComments()"
      />

      <main ref="editorScrollRoot" class="min-h-0 min-w-0 flex-1 overflow-y-auto thin-scrollbar p-8">
        <!--
          TOC sits beside the document inside the same scroll surface —
          not a separate shell column — so the page still feels like one canvas.
        -->
        <div class="mx-auto flex w-full max-w-6xl items-start justify-center gap-10">
          <div class="min-w-0 w-full max-w-3xl">
            <p v-if="!activePage" class="text-neutral-500 dark:text-neutral-400">Select a page from the sidebar.</p>
            <Editor
              v-else
              :key="`${activePage.id}:${editorEpoch}`"
              :page-id="activePage.id"
              :workspace-id="activePage.workspace_id"
              :title="activePage.title"
              :icon="activePage.icon"
              :link-token="linkToken"
              :read-only="activePage.role === 'viewer'"
              @presence-change="presentUsers = $event"
              @editor-ready="editorInstance = $event"
              @open-comment="openComments($event)"
            />
          </div>

          <TableOfContents
            v-if="activePage"
            :editor="editorInstance"
            :scroll-root="editorScrollRoot"
          />
        </div>
      </main>
    </div>

    <ShareDialog v-if="activePage" v-model:open="shareOpen" :page-id="activePage.id" />
    <VersionHistory
      v-if="activePage && activePage.role !== 'viewer'"
      v-model:open="historyOpen"
      :page-id="activePage.id"
      @restored="onVersionRestored"
    />
    <CommentSidebar
      v-if="activePage"
      v-model:open="commentsOpen"
      :page-id="activePage.id"
      :focused-mark-id="focusedCommentMarkId"
    />
    <template v-if="authStore.workspace">
      <CreateWorkspaceModal v-model:open="createWorkspaceOpen" />
      <WorkspaceMembersModal v-model:open="membersOpen" :workspace-id="authStore.workspace.id" />
      <TrashModal v-model:open="trashOpen" :workspace-id="authStore.workspace.id" />
    </template>
  </div>
</template>
