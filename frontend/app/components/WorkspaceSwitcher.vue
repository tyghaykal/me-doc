<script setup lang="ts">
import type { Workspace } from '~/stores/workspaces'

const emit = defineEmits<{
  'open-create': []
  'open-members': []
}>()

const authStore = useAuthStore()
const workspacesStore = useWorkspacesStore()

const open = ref(false)

function select(ws: Workspace) {
  workspacesStore.setActive(ws)
  open.value = false
}

function openCreate() {
  open.value = false
  emit('open-create')
}

function openMembers() {
  open.value = false
  emit('open-members')
}
</script>

<template>
  <div class="relative">
    <button
      type="button"
      class="flex w-full items-center justify-between gap-1 rounded px-2 py-1 text-lg font-semibold text-neutral-900 hover:bg-neutral-100 dark:text-neutral-100 dark:hover:bg-neutral-800"
      @click="open = !open"
    >
      {{ authStore.workspace?.name }}
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="h-4 w-4">
        <path d="m6 9 6 6 6-6" />
      </svg>
    </button>

    <template v-if="open">
      <div class="fixed inset-0 z-40" @click="open = false" />
      <div
        role="menu"
        class="absolute left-0 z-50 mt-1 w-full rounded-md border border-neutral-200 bg-white py-1 shadow-lg dark:border-neutral-700 dark:bg-neutral-900"
      >
        <button
          v-for="ws in workspacesStore.list"
          :key="ws.id"
          type="button"
          role="menuitem"
          class="flex w-full items-center justify-between px-3 py-2 text-left text-sm text-neutral-700 hover:bg-neutral-50 dark:text-neutral-300 dark:hover:bg-neutral-800"
          @click="select(ws)"
        >
          {{ ws.name }}
          <span v-if="ws.id === authStore.workspace?.id" class="text-neutral-400">✓</span>
        </button>

        <div class="my-1 border-t border-neutral-200 dark:border-neutral-800" />

        <button
          type="button"
          role="menuitem"
          class="block w-full px-3 py-2 text-left text-sm text-neutral-700 hover:bg-neutral-50 dark:text-neutral-300 dark:hover:bg-neutral-800"
          @click="openCreate"
        >
          + New workspace
        </button>
        <button
          type="button"
          role="menuitem"
          class="block w-full px-3 py-2 text-left text-sm text-neutral-700 hover:bg-neutral-50 dark:text-neutral-300 dark:hover:bg-neutral-800"
          @click="openMembers"
        >
          Manage members
        </button>
      </div>
    </template>
  </div>
</template>
