<script setup lang="ts">
const props = defineProps<{
  open: boolean
}>()

const emit = defineEmits<{
  'update:open': [value: boolean]
}>()

const api = useApi()
const config = useRuntimeConfig()
const minioBase = config.public.minioBase

interface Me {
  id: string
  email: string
  display_name: string | null
  avatar_key: string | null
}

const displayName = ref('')
const avatarKey = ref<string | null>(null)
const nameSaving = ref(false)
const nameError = ref<string | null>(null)
const nameOk = ref(false)

const avatarUploading = ref(false)
const avatarError = ref<string | null>(null)

const currentPassword = ref('')
const newPassword = ref('')
const confirmPassword = ref('')
const pwSaving = ref(false)
const pwMessage = ref<{ ok: boolean; text: string } | null>(null)

const avatarUrl = computed(() =>
  avatarKey.value ? `${minioBase}/${avatarKey.value}` : null,
)

function errText(err: any, fallback: string): string {
  return err?.data?.message ?? err?.message ?? fallback
}

async function loadMe() {
  nameError.value = null
  try {
    const me = await api<Me>('/auth/me')
    displayName.value = me.display_name ?? ''
    avatarKey.value = me.avatar_key
  } catch (err: any) {
    nameError.value = errText(err, 'Failed to load profile.')
  }
}

async function saveName() {
  nameError.value = null
  nameOk.value = false
  nameSaving.value = true
  try {
    await api('/auth/me', { method: 'PATCH', body: { display_name: displayName.value } })
    nameOk.value = true
  } catch (err: any) {
    nameError.value = errText(err, 'Failed to save name.')
  } finally {
    nameSaving.value = false
  }
}

async function onAvatarChange(event: Event) {
  const file = (event.target as HTMLInputElement).files?.[0]
  if (!file) return
  avatarError.value = null
  avatarUploading.value = true
  try {
    const { upload_url, s3_key } = await api<{ upload_url: string; s3_key: string }>(
      '/auth/me/avatar/presign',
      { method: 'POST', body: { filename: file.name, content_type: file.type } },
    )
    await fetch(upload_url, { method: 'PUT', body: file })
    avatarKey.value = s3_key
  } catch (err: any) {
    avatarError.value = errText(err, 'Failed to upload avatar.')
  } finally {
    avatarUploading.value = false
  }
}

async function changePassword() {
  pwMessage.value = null
  if (newPassword.value !== confirmPassword.value) {
    pwMessage.value = { ok: false, text: 'Passwords do not match.' }
    return
  }
  pwSaving.value = true
  try {
    await api('/auth/me/password', {
      method: 'POST',
      body: { current_password: currentPassword.value, new_password: newPassword.value },
    })
    pwMessage.value = { ok: true, text: 'Password changed.' }
    currentPassword.value = ''
    newPassword.value = ''
    confirmPassword.value = ''
  } catch (err: any) {
    pwMessage.value = { ok: false, text: errText(err, 'Failed to change password.') }
  } finally {
    pwSaving.value = false
  }
}

function close() {
  emit('update:open', false)
}

watch(
  () => props.open,
  (isOpen) => {
    if (isOpen) loadMe()
  },
)
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
        aria-label="User settings"
        class="w-full max-w-md rounded-lg bg-white p-6 shadow-xl dark:bg-slate-900"
        @keydown.esc="close"
      >
        <div class="flex items-start justify-between">
          <h2 class="text-xl font-bold text-slate-900 dark:text-slate-100">Settings</h2>
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
          <h3 class="text-sm font-semibold text-slate-700 dark:text-slate-300">Display name</h3>
          <form class="mt-2 flex gap-2" @submit.prevent="saveName">
            <input
              v-model="displayName"
              type="text"
              placeholder="Your name"
              class="flex-1 rounded border border-slate-300 px-3 py-2 text-sm dark:border-slate-700 dark:bg-slate-800 dark:text-slate-100"
            />
            <button
              type="submit"
              :disabled="nameSaving"
              class="rounded bg-slate-900 px-3 py-2 text-sm font-medium text-white disabled:opacity-50 dark:bg-slate-100 dark:text-slate-900"
            >
              {{ nameSaving ? '…' : 'Save' }}
            </button>
          </form>
          <p v-if="nameError" class="mt-2 text-sm text-red-600 dark:text-red-400">{{ nameError }}</p>
          <p v-else-if="nameOk" class="mt-2 text-sm text-green-600 dark:text-green-400">Saved.</p>
        </section>

        <section class="mt-6 border-t border-slate-200 pt-5 dark:border-slate-800">
          <h3 class="text-sm font-semibold text-slate-700 dark:text-slate-300">Avatar</h3>
          <div class="mt-2 flex items-center gap-3">
            <img
              v-if="avatarUrl"
              :src="avatarUrl"
              alt="Avatar"
              class="h-12 w-12 rounded-full object-cover"
            />
            <input
              type="file"
              accept="image/*"
              :disabled="avatarUploading"
              class="text-sm text-slate-700 dark:text-slate-300"
              @change="onAvatarChange"
            />
          </div>
          <p v-if="avatarUploading" class="mt-2 text-sm text-slate-500 dark:text-slate-400">Uploading…</p>
          <p v-if="avatarError" class="mt-2 text-sm text-red-600 dark:text-red-400">{{ avatarError }}</p>
        </section>

        <section class="mt-6 border-t border-slate-200 pt-5 dark:border-slate-800">
          <h3 class="text-sm font-semibold text-slate-700 dark:text-slate-300">Change password</h3>
          <form class="mt-2 space-y-2" @submit.prevent="changePassword">
            <input
              v-model="currentPassword"
              type="password"
              required
              placeholder="Current password"
              class="w-full rounded border border-slate-300 px-3 py-2 text-sm dark:border-slate-700 dark:bg-slate-800 dark:text-slate-100"
            />
            <input
              v-model="newPassword"
              type="password"
              required
              placeholder="New password"
              class="w-full rounded border border-slate-300 px-3 py-2 text-sm dark:border-slate-700 dark:bg-slate-800 dark:text-slate-100"
            />
            <input
              v-model="confirmPassword"
              type="password"
              required
              placeholder="Confirm new password"
              class="w-full rounded border border-slate-300 px-3 py-2 text-sm dark:border-slate-700 dark:bg-slate-800 dark:text-slate-100"
            />
            <button
              type="submit"
              :disabled="pwSaving"
              class="rounded bg-slate-900 px-3 py-2 text-sm font-medium text-white disabled:opacity-50 dark:bg-slate-100 dark:text-slate-900"
            >
              {{ pwSaving ? '…' : 'Change password' }}
            </button>
          </form>
          <p
            v-if="pwMessage"
            class="mt-2 text-sm"
            :class="pwMessage.ok ? 'text-green-600 dark:text-green-400' : 'text-red-600 dark:text-red-400'"
          >
            {{ pwMessage.text }}
          </p>
        </section>
      </div>
    </div>
  </Teleport>
</template>
