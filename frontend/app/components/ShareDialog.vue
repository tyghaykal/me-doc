<script setup lang="ts">
import type { ShareGrant } from '~/stores/pages'

const props = defineProps<{
  pageId: string
  open: boolean
}>()

const emit = defineEmits<{
  'update:open': [value: boolean]
}>()

const api = useApi()
const pagesStore = usePagesStore()

const shares = ref<ShareGrant[]>([])
const sharesLoading = ref(false)

async function loadShares() {
  sharesLoading.value = true
  try {
    shares.value = await pagesStore.listShares(props.pageId)
  } finally {
    sharesLoading.value = false
  }
}

watch(
  () => props.open,
  (o) => {
    if (o) loadShares()
  },
)

async function revoke(id: string) {
  await pagesStore.revokeShare(id)
  shares.value = shares.value.filter((s) => s.id !== id)
}

async function changeRole(grant: ShareGrant, role: 'viewer' | 'editor') {
  const previous = grant.role
  grant.role = role
  try {
    await pagesStore.updateShareRole(grant.id, role)
  } catch {
    grant.role = previous
  }
}

const inviteEmail = ref('')
const inviteRole = ref<'viewer' | 'editor'>('viewer')
const inviteMessage = ref<{ ok: boolean; text: string } | null>(null)
const inviting = ref(false)

const linkRole = ref<'viewer' | 'editor'>('viewer')
const linkUrl = ref<string | null>(null)
const linkError = ref<string | null>(null)
const generating = ref(false)
const copied = ref(false)

function errText(err: any, fallback: string): string {
  return err?.data?.message ?? err?.message ?? fallback
}

async function invite() {
  inviteMessage.value = null
  inviting.value = true
  const email = inviteEmail.value.trim()
  try {
    const res = await api<{ invited: boolean }>(`/pages/${props.pageId}/share`, {
      method: 'POST',
      body: { email, role: inviteRole.value },
    })
    inviteMessage.value = res.invited
      ? { ok: true, text: `${email} doesn't have an account yet — invited by email. They'll see this page once they sign up.` }
      : { ok: true, text: `Shared with ${email}.` }
    inviteEmail.value = ''
    await loadShares()
  } catch (err: any) {
    inviteMessage.value = { ok: false, text: errText(err, 'Failed to share.') }
  } finally {
    inviting.value = false
  }
}

async function generateLink() {
  linkError.value = null
  copied.value = false
  generating.value = true
  try {
    const res = await api<{ link_token: string; role: string }>(
      `/pages/${props.pageId}/share/link`,
      { method: 'POST', body: { role: linkRole.value } },
    )
    linkUrl.value = `${window.location.origin}/app/${props.pageId}?link=${res.link_token}`
    await copyLink()
  } catch (err: any) {
    linkError.value = errText(err, 'Failed to generate link.')
  } finally {
    generating.value = false
  }
}

async function copyLink() {
  if (!linkUrl.value) return
  await navigator.clipboard.writeText(linkUrl.value)
  copied.value = true
}

function close() {
  emit('update:open', false)
}
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4 font-sans dark:bg-black/60"
      @click.self="close"
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-label="Share page"
        class="w-full max-w-md rounded-lg bg-white p-6 shadow-xl dark:bg-neutral-900"
        @keydown.esc="close"
      >
        <div class="flex items-start justify-between">
          <h2 class="text-xl font-bold text-neutral-900 dark:text-neutral-100">Share</h2>
          <button
            type="button"
            aria-label="Close"
            class="text-neutral-400 hover:text-neutral-600 dark:text-neutral-500 dark:hover:text-neutral-300"
            @click="close"
          >
            ✕
          </button>
        </div>

        <section class="mt-5">
          <h3 class="text-sm font-semibold text-neutral-700 dark:text-neutral-300">Invite by email</h3>
          <form class="mt-2 flex gap-2" @submit.prevent="invite">
            <input
              v-model="inviteEmail"
              type="email"
              required
              placeholder="name@example.com"
              class="flex-1 rounded border border-neutral-300 px-3 py-2 text-sm dark:border-neutral-700 dark:bg-neutral-800 dark:text-neutral-100"
            />
            <select
              v-model="inviteRole"
              class="rounded border border-neutral-300 bg-white px-2 py-2 text-sm text-neutral-900 dark:border-neutral-700 dark:bg-neutral-800 dark:text-neutral-100"
            >
              <option value="viewer" class="bg-white text-neutral-900 dark:bg-neutral-800 dark:text-neutral-100">
                Viewer
              </option>
              <option value="editor" class="bg-white text-neutral-900 dark:bg-neutral-800 dark:text-neutral-100">
                Editor
              </option>
            </select>
            <button
              type="submit"
              :disabled="inviting"
              class="rounded bg-neutral-900 px-3 py-2 text-sm font-medium text-white disabled:opacity-50 dark:bg-neutral-100 dark:text-neutral-900"
            >
              {{ inviting ? '…' : 'Invite' }}
            </button>
          </form>
          <p
            v-if="inviteMessage"
            class="mt-2 text-sm"
            :class="inviteMessage.ok ? 'text-green-600 dark:text-green-400' : 'text-red-600 dark:text-red-400'"
          >
            {{ inviteMessage.text }}
          </p>
        </section>

        <section class="mt-6 border-t border-neutral-200 pt-5 dark:border-neutral-800">
          <h3 class="text-sm font-semibold text-neutral-700 dark:text-neutral-300">People with access</h3>
          <p v-if="sharesLoading" class="mt-2 text-sm text-neutral-400 dark:text-neutral-500">Loading…</p>
          <p
            v-else-if="!shares.filter((s) => s.principal_type === 'user').length"
            class="mt-2 text-sm text-neutral-400 dark:text-neutral-500"
          >
            No one else has access yet.
          </p>
          <ul v-else class="mt-2 space-y-1">
            <li
              v-for="grant in shares.filter((s) => s.principal_type === 'user')"
              :key="grant.id"
              class="flex items-center justify-between gap-2 rounded px-1.5 py-1 text-sm"
            >
              <span class="min-w-0 truncate text-neutral-700 dark:text-neutral-300">
                {{ grant.email }}
                <span v-if="grant.pending" class="ml-1 text-xs text-neutral-400 dark:text-neutral-500">(pending)</span>
              </span>
              <span class="flex shrink-0 items-center gap-1.5">
                <select
                  :value="grant.role"
                  class="rounded border border-neutral-300 bg-white px-1.5 py-0.5 text-xs text-neutral-900 dark:border-neutral-700 dark:bg-neutral-800 dark:text-neutral-100"
                  @change="changeRole(grant, ($event.target as HTMLSelectElement).value as 'viewer' | 'editor')"
                >
                  <option value="viewer" class="bg-white text-neutral-900 dark:bg-neutral-800 dark:text-neutral-100">
                    Viewer
                  </option>
                  <option value="editor" class="bg-white text-neutral-900 dark:bg-neutral-800 dark:text-neutral-100">
                    Editor
                  </option>
                </select>
                <button
                  type="button"
                  aria-label="Remove access"
                  title="Remove access"
                  class="flex h-6 w-6 items-center justify-center rounded text-neutral-400 hover:bg-neutral-100 hover:text-red-600 dark:text-neutral-500 dark:hover:bg-neutral-800 dark:hover:text-red-400"
                  @click="revoke(grant.id)"
                >
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    viewBox="0 0 20 20"
                    fill="currentColor"
                    class="h-3.5 w-3.5"
                    aria-hidden="true"
                  >
                    <path
                      fill-rule="evenodd"
                      d="M8.75 1A2.75 2.75 0 0 0 6 3.75v.443c-.795.077-1.584.176-2.365.298a.75.75 0 1 0 .23 1.482l.149-.022.841 10.518A2.75 2.75 0 0 0 7.596 19h4.807a2.75 2.75 0 0 0 2.742-2.53l.841-10.52.149.023a.75.75 0 0 0 .23-1.482A41.03 41.03 0 0 0 14 4.193v-.443A2.75 2.75 0 0 0 11.25 1h-2.5ZM10 4c.84 0 1.673.025 2.5.075V3.75c0-.69-.56-1.25-1.25-1.25h-2.5c-.69 0-1.25.56-1.25 1.25v.325C8.327 4.025 9.16 4 10 4ZM8.58 7.72a.75.75 0 0 0-1.5.06l.3 7.5a.75.75 0 1 0 1.5-.06l-.3-7.5Zm4.34.06a.75.75 0 1 0-1.5-.06l-.3 7.5a.75.75 0 1 0 1.5.06l.3-7.5Z"
                      clip-rule="evenodd"
                    />
                  </svg>
                </button>
              </span>
            </li>
          </ul>
        </section>

        <section class="mt-6 border-t border-neutral-200 pt-5 dark:border-neutral-800">
          <h3 class="text-sm font-semibold text-neutral-700 dark:text-neutral-300">Public link</h3>
          <div class="mt-2 flex gap-2">
            <select
              v-model="linkRole"
              class="rounded border border-neutral-300 bg-white px-2 py-2 text-sm text-neutral-900 dark:border-neutral-700 dark:bg-neutral-800 dark:text-neutral-100"
            >
              <option value="viewer" class="bg-white text-neutral-900 dark:bg-neutral-800 dark:text-neutral-100">
                Viewer
              </option>
              <option value="editor" class="bg-white text-neutral-900 dark:bg-neutral-800 dark:text-neutral-100">
                Editor
              </option>
            </select>
            <button
              type="button"
              :disabled="generating"
              class="rounded bg-neutral-900 px-3 py-2 text-sm font-medium text-white disabled:opacity-50 dark:bg-neutral-100 dark:text-neutral-900"
              @click="generateLink"
            >
              {{ generating ? '…' : 'Generate link' }}
            </button>
          </div>

          <div v-if="linkUrl" class="mt-3 flex gap-2">
            <input
              :value="linkUrl"
              readonly
              class="flex-1 rounded border border-neutral-300 bg-neutral-50 px-3 py-2 text-sm dark:border-neutral-700 dark:bg-neutral-800 dark:text-neutral-100"
              @focus="($event.target as HTMLInputElement).select()"
            />
            <button
              type="button"
              class="rounded border border-neutral-300 px-3 py-2 text-sm font-medium text-neutral-700 hover:bg-neutral-50 dark:border-neutral-700 dark:text-neutral-300 dark:hover:bg-neutral-800"
              @click="copyLink"
            >
              {{ copied ? 'Copied' : 'Copy' }}
            </button>
          </div>

          <p v-if="linkError" class="mt-2 text-sm text-red-600 dark:text-red-400">{{ linkError }}</p>
        </section>
      </div>
    </div>
  </Teleport>
</template>
