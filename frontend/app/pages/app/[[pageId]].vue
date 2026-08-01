<script setup lang="ts">
import type { Editor } from '@tiptap/vue-3'
import type { Page } from '~/stores/pages'

definePageMeta({ middleware: ['auth'] })

const authStore = useAuthStore()
const pagesStore = usePagesStore()
const route = useRoute()

const shareOpen = ref(false)
const historyOpen = ref(false)
// Bumped after a version restore so <Editor> remounts with a fresh Y.Doc that
// re-syncs from the restored page_content (and a new collab room on the server).
const editorEpoch = ref(0)
const editorLoading = ref(false)
const commentsOpen = ref(false)
const focusedCommentMarkId = ref<string | null>(null)

function onEditorReady(ed: Editor | null) {
  editorInstance.value = ed
  if (ed) editorLoading.value = false
}

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

const { record: recordRecent, rename: renameRecent } = useRecents()

// Keep the comment thread live for whichever page is open, so create/reply/
// resolve/delete from other collaborators appear without a manual refresh.
// Also carries title/icon rename events — pagesStore.patchPageMeta covers a
// normal member's page list, but two more local caches aren't part of that
// store and need the same patch by hand: an anonymous link-guest's
// `sharedPage`, and the "Recents" sidebar list (its own localStorage-backed
// snapshot, otherwise stuck showing whatever title was current when the page
// was last visited).
useCommentStream(
  () => activePage.value?.id,
  () => linkToken.value,
  (patch) => {
    if (sharedPage.value) Object.assign(sharedPage.value, patch)
    if (activePage.value) renameRecent(activePage.value.id, patch)
  },
)

// Who else is currently viewing/editing the open page, reported live by Editor's
// awareness subscription. Reset on page switch so a stale list doesn't flash.
const presentUsers = ref<
  { clientId: number; name: string; email: string | null; color: string; avatarUrl: string | null }[]
>([])
watch(
  () => activePage.value?.id,
  (id) => {
    presentUsers.value = []
    editorInstance.value = null
    editorLoading.value = !!id
  },
)

useAppShellData()

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
          <div
            class="min-w-0 w-full"
            :class="!activePage ? 'max-w-3xl' : activePage.kind === 'diagram' ? 'max-w-5xl' : 'max-w-3xl'"
          >
            <template v-if="!activePage">
              <PageHomeList
                v-if="authStore.workspace"
                :workspace-id="authStore.workspace.id"
              />
              <p v-else class="text-neutral-500 dark:text-neutral-400">Select a page from the sidebar.</p>
            </template>
            <div v-else class="relative" :class="activePage.kind === 'diagram' ? 'h-[calc(100vh-9rem)]' : ''">
              <div
                v-if="editorLoading && activePage.kind !== 'diagram'"
                class="absolute inset-0 z-10 flex min-h-[40vh] flex-col items-center justify-center gap-3 bg-white/80 dark:bg-neutral-900/80"
              >
                <div
                  class="h-8 w-8 animate-spin rounded-full border-2 border-neutral-300 border-t-neutral-800 dark:border-neutral-600 dark:border-t-neutral-200"
                  aria-hidden="true"
                />
                <p class="text-sm text-neutral-500 dark:text-neutral-400">Loading document…</p>
              </div>
              <DiagramPage
                v-if="activePage.kind === 'diagram'"
                :key="`${activePage.id}:${editorEpoch}`"
                :page-id="activePage.id"
                :workspace-id="activePage.workspace_id"
                :title="activePage.title"
                :icon="activePage.icon"
                :link-token="linkToken"
                :read-only="activePage.role === 'viewer'"
                @presence-change="presentUsers = $event"
              />
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
                @editor-ready="onEditorReady"
                @open-comment="openComments($event)"
              />
            </div>
          </div>

          <TableOfContents
            v-if="activePage && activePage.kind !== 'diagram'"
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
