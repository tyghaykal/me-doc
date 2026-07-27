<script setup lang="ts">
import type { WorkspaceMember } from '~/stores/workspaces'

const props = defineProps<{
  open: boolean
  workspaceId: string
}>()

const emit = defineEmits<{
  'update:open': [value: boolean]
}>()

const authStore = useAuthStore()
const workspacesStore = useWorkspacesStore()

const members = ref<WorkspaceMember[]>([])
const loading = ref(false)
const error = ref<string | null>(null)
const removingId = ref<string | null>(null)

const inviteEmail = ref('')
const inviteRole = ref<'admin' | 'member' | 'guest'>('member')
const inviteMessage = ref<{ ok: boolean; text: string } | null>(null)
const inviting = ref(false)

const myRole = computed(
  () => members.value.find((m) => m.user_id === authStore.user?.id)?.role ?? null,
)
const canManage = computed(() => myRole.value === 'owner' || myRole.value === 'admin')

function errText(err: any, fallback: string): string {
  return err?.data?.message ?? err?.message ?? fallback
}

async function load() {
  loading.value = true
  error.value = null
  try {
    members.value = await workspacesStore.fetchMembers(props.workspaceId)
  } catch (err: any) {
    error.value = errText(err, 'Failed to load members.')
  } finally {
    loading.value = false
  }
}

async function invite() {
  inviteMessage.value = null
  inviting.value = true
  try {
    await workspacesStore.addMember(props.workspaceId, inviteEmail.value.trim(), inviteRole.value)
    inviteMessage.value = { ok: true, text: `Added ${inviteEmail.value.trim()}.` }
    inviteEmail.value = ''
    await load()
  } catch (err: any) {
    inviteMessage.value = { ok: false, text: errText(err, 'Failed to add member.') }
  } finally {
    inviting.value = false
  }
}

async function remove(userId: string) {
  removingId.value = userId
  error.value = null
  try {
    await workspacesStore.removeMember(props.workspaceId, userId)
    await load()
  } catch (err: any) {
    error.value = errText(err, 'Failed to remove member.')
  } finally {
    removingId.value = null
  }
}

function close() {
  emit('update:open', false)
}

watch(
  () => props.open,
  (isOpen) => {
    if (isOpen) load()
  },
)
</script>

<template>
  <Teleport to="body">
    <Transition
      enter-active-class="transition duration-200 ease-out"
      enter-from-class="opacity-0"
      enter-to-class="opacity-100"
      leave-active-class="transition duration-150 ease-in"
      leave-from-class="opacity-100"
      leave-to-class="opacity-0"
    >
    <div
      v-if="open"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4 font-sans dark:bg-black/60"
      @click.self="close"
    >
      <Transition
        appear
        enter-active-class="transition duration-200 ease-out"
        enter-from-class="opacity-0 scale-95 translate-y-2"
        enter-to-class="opacity-100 scale-100 translate-y-0"
        leave-active-class="transition duration-150 ease-in"
        leave-from-class="opacity-100 scale-100 translate-y-0"
        leave-to-class="opacity-0 scale-95 translate-y-2"
      >
      <div
        role="dialog"
        aria-modal="true"
        aria-label="Workspace members"
        class="w-full max-w-md rounded-lg bg-white p-6 shadow-xl dark:bg-neutral-900"
        @keydown.esc="close"
      >
        <div class="flex items-start justify-between">
          <h2 class="text-xl font-bold text-neutral-900 dark:text-neutral-100">Members</h2>
          <button
            type="button"
            aria-label="Close"
            class="text-neutral-400 hover:text-neutral-600 dark:text-neutral-500 dark:hover:text-neutral-300"
            @click="close"
          >
            ✕
          </button>
        </div>

        <p v-if="loading" class="mt-5 text-sm text-neutral-500 dark:text-neutral-400">Loading…</p>
        <p v-else-if="error" class="mt-5 text-sm text-red-600 dark:text-red-400">{{ error }}</p>

        <ul v-else class="mt-5 divide-y divide-neutral-200 dark:divide-neutral-800">
          <li
            v-for="m in members"
            :key="m.user_id"
            class="flex items-center justify-between py-3"
          >
            <div class="min-w-0">
              <p class="truncate text-sm text-neutral-700 dark:text-neutral-300">{{ m.email }}</p>
              <span class="text-xs text-neutral-400">{{ m.role }}</span>
            </div>
            <button
              v-if="m.user_id === authStore.user?.id"
              type="button"
              :disabled="removingId === m.user_id"
              class="rounded border border-neutral-300 px-3 py-1.5 text-sm font-medium text-neutral-700 hover:bg-neutral-50 disabled:opacity-50 dark:border-neutral-700 dark:text-neutral-300 dark:hover:bg-neutral-800"
              @click="remove(m.user_id)"
            >
              {{ removingId === m.user_id ? '…' : 'Leave' }}
            </button>
            <button
              v-else-if="canManage"
              type="button"
              :disabled="removingId === m.user_id"
              class="rounded border border-neutral-300 px-3 py-1.5 text-sm font-medium text-neutral-700 hover:bg-neutral-50 disabled:opacity-50 dark:border-neutral-700 dark:text-neutral-300 dark:hover:bg-neutral-800"
              @click="remove(m.user_id)"
            >
              {{ removingId === m.user_id ? '…' : 'Remove' }}
            </button>
          </li>
        </ul>

        <section v-if="canManage" class="mt-6 border-t border-neutral-200 pt-5 dark:border-neutral-800">
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
              class="rounded border border-neutral-300 px-2 py-2 text-sm dark:border-neutral-700 dark:bg-neutral-800 dark:text-neutral-100"
            >
              <option value="admin">Admin</option>
              <option value="member">Member</option>
              <option value="guest">Guest</option>
            </select>
            <button
              type="submit"
              :disabled="inviting"
              class="rounded bg-teal-600 px-3 py-2 text-sm font-medium text-white hover:bg-teal-700 disabled:opacity-50 dark:bg-teal-500 dark:text-neutral-950 dark:hover:bg-teal-400"
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
      </div>
      </Transition>
    </div>
    </Transition>
  </Teleport>
</template>
