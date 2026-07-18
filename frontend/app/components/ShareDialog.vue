<script setup lang="ts">
const props = defineProps<{
  pageId: string
  open: boolean
}>()

const emit = defineEmits<{
  'update:open': [value: boolean]
}>()

const api = useApi()

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
  try {
    await api(`/pages/${props.pageId}/share`, {
      method: 'POST',
      body: { email: inviteEmail.value.trim(), role: inviteRole.value },
    })
    inviteMessage.value = { ok: true, text: `Shared with ${inviteEmail.value.trim()}.` }
    inviteEmail.value = ''
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
    // ponytail: /app/pages/:id?link= is a guess — page-view route isn't built yet.
    linkUrl.value = `${window.location.origin}/app/pages/${props.pageId}?link=${res.link_token}`
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
        class="w-full max-w-md rounded-lg bg-white p-6 shadow-xl dark:bg-slate-900"
        @keydown.esc="close"
      >
        <div class="flex items-start justify-between">
          <h2 class="text-xl font-bold text-slate-900 dark:text-slate-100">Share</h2>
          <button
            type="button"
            aria-label="Close"
            class="text-slate-400 hover:text-slate-600 dark:text-slate-500 dark:hover:text-slate-300"
            @click="close"
          >
            ✕
          </button>
        </div>

        <section class="mt-5">
          <h3 class="text-sm font-semibold text-slate-700 dark:text-slate-300">Invite by email</h3>
          <form class="mt-2 flex gap-2" @submit.prevent="invite">
            <input
              v-model="inviteEmail"
              type="email"
              required
              placeholder="name@example.com"
              class="flex-1 rounded border border-slate-300 px-3 py-2 text-sm dark:border-slate-700 dark:bg-slate-800 dark:text-slate-100"
            />
            <select
              v-model="inviteRole"
              class="rounded border border-slate-300 px-2 py-2 text-sm dark:border-slate-700 dark:bg-slate-800 dark:text-slate-100"
            >
              <option value="viewer">Viewer</option>
              <option value="editor">Editor</option>
            </select>
            <button
              type="submit"
              :disabled="inviting"
              class="rounded bg-slate-900 px-3 py-2 text-sm font-medium text-white disabled:opacity-50 dark:bg-slate-100 dark:text-slate-900"
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

        <section class="mt-6 border-t border-slate-200 pt-5 dark:border-slate-800">
          <h3 class="text-sm font-semibold text-slate-700 dark:text-slate-300">Public link</h3>
          <div class="mt-2 flex gap-2">
            <select
              v-model="linkRole"
              class="rounded border border-slate-300 px-2 py-2 text-sm dark:border-slate-700 dark:bg-slate-800 dark:text-slate-100"
            >
              <option value="viewer">Viewer</option>
              <option value="editor">Editor</option>
            </select>
            <button
              type="button"
              :disabled="generating"
              class="rounded bg-slate-900 px-3 py-2 text-sm font-medium text-white disabled:opacity-50 dark:bg-slate-100 dark:text-slate-900"
              @click="generateLink"
            >
              {{ generating ? '…' : 'Generate link' }}
            </button>
          </div>

          <div v-if="linkUrl" class="mt-3 flex gap-2">
            <input
              :value="linkUrl"
              readonly
              class="flex-1 rounded border border-slate-300 bg-slate-50 px-3 py-2 text-sm dark:border-slate-700 dark:bg-slate-800 dark:text-slate-100"
              @focus="($event.target as HTMLInputElement).select()"
            />
            <button
              type="button"
              class="rounded border border-slate-300 px-3 py-2 text-sm font-medium text-slate-700 hover:bg-slate-50 dark:border-slate-700 dark:text-slate-300 dark:hover:bg-slate-800"
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
