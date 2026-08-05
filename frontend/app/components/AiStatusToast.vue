<script setup lang="ts">
import type { AiTokenUsage } from '~/composables/useAiStatus'

const { status } = useAiStatus()

function tokenLabel(tokens: AiTokenUsage): string {
  if (!tokens) return ''
  if (tokens.total) return ` ≈ ${tokens.total.toLocaleString()} tokens`
  if (tokens.prompt + tokens.completion) return ` ≈ ${(tokens.prompt + tokens.completion).toLocaleString()} tokens`
  return ''
}
</script>

<template>
  <Teleport to="body">
    <Transition
      enter-active-class="transition duration-200 ease-out"
      enter-from-class="translate-y-2 opacity-0"
      leave-active-class="transition duration-150 ease-in"
      leave-to-class="translate-y-2 opacity-0"
    >
      <div
        v-if="status.kind !== 'idle'"
        role="status"
        aria-live="polite"
        class="fixed bottom-4 right-4 z-[100] flex max-w-sm items-start gap-2.5 rounded-md border border-neutral-200 bg-white px-3 py-2.5 text-sm shadow-lg dark:border-neutral-700 dark:bg-neutral-900"
      >
        <span
          v-if="status.kind === 'loading'"
          class="mt-0.5 h-3.5 w-3.5 shrink-0 animate-spin rounded-full border-2 border-neutral-300 border-t-teal-600 dark:border-neutral-700 dark:border-t-teal-400"
        />
        <svg
          v-else-if="status.kind === 'success'"
          class="mt-0.5 h-4 w-4 shrink-0 text-teal-600 dark:text-teal-400"
          xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"
        >
          <path d="M20 6 9 17l-5-5" />
        </svg>
        <svg
          v-else
          class="mt-0.5 h-4 w-4 shrink-0 text-red-600 dark:text-red-400"
          xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"
        >
          <circle cx="12" cy="12" r="10" /><path d="M12 8v4M12 16h.01" />
        </svg>

        <div class="min-w-0">
          <p class="leading-tight text-neutral-800 dark:text-neutral-100">
            <template v-if="status.kind === 'loading'">{{ status.label ?? 'AI working…' }}</template>
            <template v-else>{{ status.message }}</template>
          </p>
          <p v-if="status.kind === 'success' && tokenLabel(status.tokens)" class="mt-0.5 text-xs text-neutral-500 dark:text-neutral-400">
            {{ tokenLabel(status.tokens) }}
          </p>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>
