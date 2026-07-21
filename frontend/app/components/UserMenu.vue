<script setup lang="ts">
const api = useApi()
const authStore = useAuthStore()
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
const open = ref(false)
const settingsOpen = ref(false)

const email = computed(() => authStore.user?.email ?? '')
const label = computed(() => displayName.value || email.value)
const initial = computed(() => (label.value ? label.value[0]!.toUpperCase() : '?'))
const avatarUrl = computed(() => (avatarKey.value ? `${minioBase}/${avatarKey.value}` : null))

async function loadMe() {
  try {
    const me = await api<Me>('/auth/me')
    displayName.value = me.display_name ?? ''
    avatarKey.value = me.avatar_key
  } catch {
    // Sidebar chrome — fall back to the email already in the auth store.
  }
}

onMounted(loadMe)

watch(settingsOpen, (isOpen) => {
  if (!isOpen) loadMe()
})

function openSettings() {
  open.value = false
  settingsOpen.value = true
}

async function logout() {
  open.value = false
  await authStore.logout()
  navigateTo('/login')
}
</script>

<template>
  <div class="relative">
    <button
      type="button"
      class="flex w-full items-center gap-2 rounded p-1.5 text-left hover:bg-neutral-100 dark:hover:bg-neutral-800"
      @click="open = !open"
    >
      <img
        v-if="avatarUrl"
        :src="avatarUrl"
        alt="Avatar"
        class="h-7 w-7 shrink-0 rounded-full object-cover"
      />
      <span
        v-else
        class="flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-neutral-200 text-xs font-semibold text-neutral-600 dark:bg-neutral-700 dark:text-neutral-300"
      >
        {{ initial }}
      </span>
      <span class="min-w-0 flex-1">
        <span class="block truncate text-sm font-medium text-neutral-900 dark:text-neutral-100">
          {{ label }}
        </span>
        <span v-if="displayName" class="block truncate text-xs text-neutral-500 dark:text-neutral-400">
          {{ email }}
        </span>
      </span>
    </button>

    <template v-if="open">
      <div class="fixed inset-0 z-40" @click="open = false" />
      <div
        role="menu"
        class="absolute bottom-full left-0 z-50 mb-1 w-full rounded-md border border-neutral-200 bg-white py-1 shadow-lg dark:border-neutral-700 dark:bg-neutral-900"
      >
        <button
          type="button"
          role="menuitem"
          class="block w-full px-3 py-2 text-left text-sm text-neutral-700 hover:bg-neutral-50 dark:text-neutral-300 dark:hover:bg-neutral-800"
          @click="openSettings"
        >
          Update information
        </button>
        <button
          type="button"
          role="menuitem"
          class="block w-full px-3 py-2 text-left text-sm text-neutral-700 hover:bg-neutral-50 dark:text-neutral-300 dark:hover:bg-neutral-800"
          @click="logout"
        >
          Log out
        </button>
      </div>
    </template>

    <UserSettingsModal v-model:open="settingsOpen" />
  </div>
</template>
