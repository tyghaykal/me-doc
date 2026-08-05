<script setup lang="ts">
import { DEFAULT_PAGE_ICON, type Page } from '~/stores/pages'

const props = defineProps<{
  activePage: Page | null
  presentUsers?: { clientId: number; name: string; email: string | null; color: string; avatarUrl: string | null }[]
}>()

const emit = defineEmits<{
  'open-share': []
  'open-history': []
  'open-comments': []
}>()

const authStore = useAuthStore()
const pagesStore = usePagesStore()
const menuOpen = ref(false)
const viewerMenuOpen = ref(false)
const copied = ref(false)

const selfName = computed(() => authStore.user?.display_name || authStore.user?.email || '')
const selfAvatarUrl = computed(() => resolveAvatarUrl(authStore.user?.avatar_key))

// The signed-in user's own account, open elsewhere (another tab/window, or a
// second device), still shows up as a separate collaborator in `presentUsers`
// — same email, distinct awareness client. Drop those so the topbar never
// shows both the "you" chip and a duplicate avatar for the same person.
const otherPresentUsers = computed(() => {
  const selfEmail = authStore.user?.email?.toLowerCase()
  const list = props.presentUsers ?? []
  if (!selfEmail) return list
  return list.filter((u) => u.email?.toLowerCase() !== selfEmail)
})

// Broken/inaccessible avatar URLs (e.g. a stale MinIO host:port after a
// config change) fall back to the initials chip instead of a broken-image
// icon. Keyed by clientId, with a dedicated key for the "you" chip.
const failedAvatars = ref(new Set<number | 'self'>())
function markAvatarFailed(key: number | 'self') {
  failedAvatars.value.add(key)
  // Set mutation alone doesn't trigger Vue reactivity — replace to notify.
  failedAvatars.value = new Set(failedAvatars.value)
}

// If the avatar URL changes (e.g. MinIO base fixed, or a new avatar uploaded),
// clear the failure so we try the new URL instead of permanently sticking on
// the initials chip.
watch(selfAvatarUrl, () => {
  if (failedAvatars.value.has('self')) {
    failedAvatars.value.delete('self')
    failedAvatars.value = new Set(failedAvatars.value)
  }
})
watch(
  () => otherPresentUsers.value.map((u) => `${u.clientId}:${u.avatarUrl}`).join('|'),
  () => {
    const liveIds = new Set(otherPresentUsers.value.map((u) => u.clientId))
    let changed = false
    for (const key of failedAvatars.value) {
      if (key !== 'self' && !liveIds.has(key)) {
        failedAvatars.value.delete(key)
        changed = true
      }
    }
    if (changed) failedAvatars.value = new Set(failedAvatars.value)
  },
)

// Same stable-ish hash as Editor.vue's userColor(), so a user's "you" chip
// matches the color remote peers see for them in CollaborationCaret.
function userColorFor(id: string): string {
  let hash = 0
  for (let i = 0; i < id.length; i++) hash = (hash * 31 + id.charCodeAt(i)) | 0
  return `hsl(${Math.abs(hash) % 360}, 70%, 50%)`
}

const isViewer = computed(() => props.activePage?.role === 'viewer')

// SidebarFavorites (always mounted alongside this component under
// [[pageId]].vue) owns fetching favoritePages; we just read the shared store.
const isFavorited = computed(
  () => !!props.activePage && pagesStore.favoritePages.some((p) => p.id === props.activePage!.id),
)

function toggleFavorite() {
  if (!props.activePage) return
  if (isFavorited.value) pagesStore.unfavoritePage(props.activePage.id)
  else pagesStore.favoritePage(props.activePage.id)
}

function relativeTime(iso: string): string {
  const secs = Math.round((Date.now() - new Date(iso).getTime()) / 1000)
  if (secs < 60) return 'just now'
  const mins = Math.round(secs / 60)
  if (mins < 60) return `${mins} minute${mins === 1 ? '' : 's'} ago`
  const hours = Math.round(mins / 60)
  if (hours < 24) return `${hours} hour${hours === 1 ? '' : 's'} ago`
  const days = Math.round(hours / 24)
  return `${days} day${days === 1 ? '' : 's'} ago`
}

async function copyLink() {
  await navigator.clipboard.writeText(location.href)
  copied.value = true
  setTimeout(() => (copied.value = false), 1500)
}

function openHistory() {
  menuOpen.value = false
  emit('open-history')
}

function duplicate() {
  menuOpen.value = false
  if (props.activePage) pagesStore.duplicatePage(props.activePage.id)
}
</script>

<template>
  <header
    v-if="activePage"
    class="flex items-center justify-between gap-2 border-b border-neutral-200 py-3 pl-12 pr-3 sm:pl-6 sm:pr-6 dark:border-neutral-800"
  >
    <div class="flex min-w-0 items-center gap-2 text-sm">
      <button
        v-if="authStore.isAuthenticated && !isViewer"
        type="button"
        class="hidden items-center gap-1 rounded px-1.5 py-1 text-neutral-500 hover:bg-neutral-100 sm:flex dark:text-neutral-400 dark:hover:bg-neutral-800"
        title="Visibility"
        @click="emit('open-share')"
      >
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="h-3.5 w-3.5">
          <rect x="4" y="10" width="16" height="10" rx="2" /><path d="M8 10V7a4 4 0 0 1 8 0v3" />
        </svg>
        Private
      </button>
      <span v-else class="hidden items-center gap-1 px-1.5 py-1 text-teal-700 sm:flex dark:text-teal-400">
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="h-3.5 w-3.5">
          <rect x="4" y="10" width="16" height="10" rx="2" /><path d="M8 10V7a4 4 0 0 1 8 0v3" />
        </svg>
        Shared with you
      </span>
      <span class="shrink-0">{{ activePage.icon || DEFAULT_PAGE_ICON }}</span>
      <span class="min-w-0 truncate font-medium text-neutral-900 dark:text-neutral-100">{{ activePage.title || 'Untitled' }}</span>
    </div>

    <div class="flex shrink-0 items-center gap-3">
      <div v-if="authStore.isAuthenticated" class="hidden items-center -space-x-2 sm:flex">
        <img
          v-if="selfAvatarUrl && !failedAvatars.has('self')"
          :src="selfAvatarUrl"
          :title="`${selfName} (you)`"
          class="h-6 w-6 rounded-full border-2 border-white object-cover dark:border-neutral-900"
          @error="markAvatarFailed('self')"
        />
        <span
          v-else
          :title="`${selfName} (you)`"
          class="flex h-6 w-6 items-center justify-center rounded-full border-2 border-white text-[10px] font-semibold uppercase text-white dark:border-neutral-900"
          :style="{ backgroundColor: userColorFor(authStore.user?.id ?? '') }"
        >
          {{ selfName.charAt(0) || '?' }}
        </span>

        <template v-for="u in otherPresentUsers.slice(0, 5)" :key="u.clientId">
          <img
            v-if="u.avatarUrl && !failedAvatars.has(u.clientId)"
            :src="u.avatarUrl"
            :title="u.name"
            class="h-6 w-6 rounded-full border-2 border-white object-cover dark:border-neutral-900"
            @error="markAvatarFailed(u.clientId)"
          />
          <span
            v-else
            :title="u.name"
            class="flex h-6 w-6 items-center justify-center rounded-full border-2 border-white text-[10px] font-semibold uppercase text-white dark:border-neutral-900"
            :style="{ backgroundColor: u.color }"
          >
            {{ u.name.charAt(0) || '?' }}
          </span>
        </template>
      </div>

      <div class="hidden items-center gap-1 sm:flex">
        <button
          v-if="!isViewer"
          type="button"
          class="hidden rounded px-1.5 py-0.5 text-xs text-neutral-400 transition-colors hover:bg-neutral-100 hover:text-neutral-700 sm:block dark:text-neutral-500 dark:hover:bg-neutral-800 dark:hover:text-neutral-200"
          title="View version history"
          @click="emit('open-history')"
        >
          Edited {{ relativeTime(activePage.updated_at) }}
        </button>
        <span
          v-else
          class="hidden px-1.5 py-0.5 text-xs text-neutral-400 sm:block dark:text-neutral-500"
        >
          Edited {{ relativeTime(activePage.updated_at) }}
        </span>

        <button
          v-if="authStore.isAuthenticated"
          type="button"
          aria-label="Comments"
          title="Comments"
          class="rounded p-1.5 text-neutral-500 hover:bg-neutral-100 dark:text-neutral-400 dark:hover:bg-neutral-800"
          @click="emit('open-comments')"
        >
          💬
        </button>

        <button
          v-if="authStore.isAuthenticated"
          type="button"
          :aria-label="isFavorited ? 'Remove from favorites' : 'Add to favorites'"
          class="rounded p-1.5 transition-transform hover:bg-neutral-100 active:scale-90 dark:hover:bg-neutral-800"
          :class="isFavorited ? 'text-yellow-500' : 'text-neutral-500 dark:text-neutral-400'"
          @click="toggleFavorite"
        >
          <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" :fill="isFavorited ? 'currentColor' : 'none'" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="h-4 w-4">
            <path d="m12 2 3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01z" />
          </svg>
        </button>

        <button
          type="button"
          aria-label="Copy link"
          class="rounded p-1.5 text-neutral-500 hover:bg-neutral-100 dark:text-neutral-400 dark:hover:bg-neutral-800"
          @click="copyLink"
        >
          <Transition
            mode="out-in"
            enter-active-class="transition duration-150 ease-out"
            enter-from-class="opacity-0 scale-75"
            enter-to-class="opacity-100 scale-100"
            leave-active-class="transition duration-150 ease-in"
            leave-from-class="opacity-100 scale-100"
            leave-to-class="opacity-0 scale-75"
          >
            <svg v-if="!copied" key="link" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="h-4 w-4">
              <path d="M9 17H7a5 5 0 0 1 0-10h2M15 7h2a5 5 0 0 1 0 10h-2M8 12h8" />
            </svg>
            <svg v-else key="check" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="h-4 w-4">
              <path d="M20 6 9 17l-5-5" />
            </svg>
          </Transition>
        </button>
      </div>

      <template v-if="authStore.isAuthenticated && !isViewer">
        <div class="flex items-center gap-2 border-l border-neutral-200 pl-3 dark:border-neutral-700">
          <ExportMenu :page-id="activePage.id" />

          <button
            class="rounded bg-teal-600 px-3 py-1.5 text-sm font-medium text-white hover:bg-teal-700 dark:bg-teal-500 dark:text-neutral-950 dark:hover:bg-teal-400"
            @click="emit('open-share')"
          >
            Share
          </button>

          <div class="relative">
            <button
              type="button"
              aria-label="More"
              class="rounded p-1.5 text-neutral-500 hover:bg-neutral-100 dark:text-neutral-400 dark:hover:bg-neutral-800"
              @click="menuOpen = !menuOpen"
            >
              <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" class="h-4 w-4">
                <circle cx="5" cy="12" r="1.5" /><circle cx="12" cy="12" r="1.5" /><circle cx="19" cy="12" r="1.5" />
              </svg>
            </button>

            <template v-if="menuOpen">
              <div class="fixed inset-0 z-40" @click="menuOpen = false" />
              <div
                role="menu"
                class="absolute right-0 z-50 mt-1 w-44 rounded-md border border-neutral-200 bg-white py-1 shadow-lg dark:border-neutral-700 dark:bg-neutral-900"
              >
                <button
                  type="button"
                  role="menuitem"
                  class="block w-full px-3 py-1.5 text-left text-sm text-neutral-700 hover:bg-neutral-50 dark:text-neutral-300 dark:hover:bg-neutral-800"
                  @click="openHistory"
                >
                  History
                </button>
                <button
                  type="button"
                  role="menuitem"
                  class="block w-full px-3 py-1.5 text-left text-sm text-neutral-700 hover:bg-neutral-50 dark:text-neutral-300 dark:hover:bg-neutral-800"
                  @click="duplicate"
                >
                  Duplicate
                </button>
                <div class="my-1 border-t border-neutral-200 dark:border-neutral-700 sm:hidden" />
                <button
                  v-if="authStore.isAuthenticated"
                  type="button"
                  role="menuitem"
                  class="flex w-full items-center gap-2 px-3 py-1.5 text-left text-sm text-neutral-700 hover:bg-neutral-50 sm:hidden dark:text-neutral-300 dark:hover:bg-neutral-800"
                  @click="menuOpen = false; emit('open-comments')"
                >
                  💬 Comments
                </button>
                <button
                  v-if="authStore.isAuthenticated"
                  type="button"
                  role="menuitem"
                  class="flex w-full items-center gap-2 px-3 py-1.5 text-left text-sm text-neutral-700 hover:bg-neutral-50 sm:hidden dark:text-neutral-300 dark:hover:bg-neutral-800"
                  @click="toggleFavorite; menuOpen = false"
                >
                  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" :fill="isFavorited ? 'currentColor' : 'none'" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="h-4 w-4">
                    <path d="m12 2 3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01z" />
                  </svg>
                  {{ isFavorited ? 'Remove from favorites' : 'Add to favorites' }}
                </button>
                <button
                  type="button"
                  role="menuitem"
                  class="flex w-full items-center gap-2 px-3 py-1.5 text-left text-sm text-neutral-700 hover:bg-neutral-50 sm:hidden dark:text-neutral-300 dark:hover:bg-neutral-800"
                  @click="menuOpen = false; copyLink()"
                >
                  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="h-4 w-4">
                    <path d="M9 17H7a5 5 0 0 1 0-10h2M15 7h2a5 5 0 0 1 0 10h-2M8 12h8" />
                  </svg>
                  Copy link
                </button>
              </div>
            </template>
          </div>
        </div>
      </template>

      <!-- Mobile-only overflow for viewers / link guests (no Share/Export/More
           block above): keeps Comments / Favorite / Copy link reachable without
           cluttering the narrow topbar. -->
      <div v-else-if="authStore.isAuthenticated" class="relative sm:hidden">
        <button
          type="button"
          aria-label="More"
          class="rounded p-1.5 text-neutral-500 hover:bg-neutral-100 dark:text-neutral-400 dark:hover:bg-neutral-800"
          @click="viewerMenuOpen = !viewerMenuOpen"
        >
          <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" class="h-4 w-4">
            <circle cx="5" cy="12" r="1.5" /><circle cx="12" cy="12" r="1.5" /><circle cx="19" cy="12" r="1.5" />
          </svg>
        </button>

        <template v-if="viewerMenuOpen">
          <div class="fixed inset-0 z-40" @click="viewerMenuOpen = false" />
          <div
            role="menu"
            class="absolute right-0 z-50 mt-1 w-44 rounded-md border border-neutral-200 bg-white py-1 shadow-lg dark:border-neutral-700 dark:bg-neutral-900"
          >
            <button
              type="button"
              role="menuitem"
              class="flex w-full items-center gap-2 px-3 py-1.5 text-left text-sm text-neutral-700 hover:bg-neutral-50 dark:text-neutral-300 dark:hover:bg-neutral-800"
              @click="viewerMenuOpen = false; emit('open-comments')"
            >
              💬 Comments
            </button>
            <button
              type="button"
              role="menuitem"
              class="flex w-full items-center gap-2 px-3 py-1.5 text-left text-sm text-neutral-700 hover:bg-neutral-50 dark:text-neutral-300 dark:hover:bg-neutral-800"
              @click="toggleFavorite; viewerMenuOpen = false"
            >
              <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" :fill="isFavorited ? 'currentColor' : 'none'" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="h-4 w-4">
                <path d="m12 2 3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01z" />
              </svg>
              {{ isFavorited ? 'Remove from favorites' : 'Add to favorites' }}
            </button>
            <button
              type="button"
              role="menuitem"
              class="flex w-full items-center gap-2 px-3 py-1.5 text-left text-sm text-neutral-700 hover:bg-neutral-50 dark:text-neutral-300 dark:hover:bg-neutral-800"
              @click="viewerMenuOpen = false; copyLink()"
            >
              <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="h-4 w-4">
                <path d="M9 17H7a5 5 0 0 1 0-10h2M15 7h2a5 5 0 0 1 0 10h-2M8 12h8" />
              </svg>
              Copy link
            </button>
          </div>
        </template>
      </div>
    </div>
  </header>
</template>
