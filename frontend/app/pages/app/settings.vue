<script setup lang="ts">
definePageMeta({ middleware: ['auth'] })

const api = useApi()

interface AiSettings {
  api_url: string
  model: string
  has_key: boolean
}

const apiUrl = ref('')
const model = ref('')
const apiKey = ref('')
const hasKey = ref(false)
const loading = ref(true)
const saving = ref(false)
const message = ref<{ ok: boolean; text: string } | null>(null)

function errText(err: any, fallback: string): string {
  return err?.data?.message ?? err?.message ?? fallback
}

async function load() {
  try {
    const s = await api<AiSettings>('/ai/settings')
    apiUrl.value = s.api_url
    model.value = s.model
    hasKey.value = s.has_key
  } catch (err: any) {
    message.value = { ok: false, text: errText(err, 'Failed to load AI settings.') }
  } finally {
    loading.value = false
  }
}

async function save() {
  message.value = null
  saving.value = true
  try {
    // Omitting api_key tells the backend to keep the key already stored — the
    // field is never pre-filled, so a blank box means "unchanged", not "clear".
    const s = await api<AiSettings>('/ai/settings', {
      method: 'PUT',
      body: {
        api_url: apiUrl.value,
        model: model.value,
        ...(apiKey.value ? { api_key: apiKey.value } : {}),
      },
    })
    hasKey.value = s.has_key
    apiKey.value = ''
    message.value = { ok: true, text: 'Saved.' }
  } catch (err: any) {
    message.value = { ok: false, text: errText(err, 'Failed to save AI settings.') }
  } finally {
    saving.value = false
  }
}

onMounted(load)
</script>

<template>
  <div class="min-h-screen bg-neutral-50 px-4 py-10 font-sans dark:bg-neutral-950">
    <div class="mx-auto w-full max-w-md rounded-lg bg-white p-6 shadow-xl dark:bg-neutral-900">
      <div class="flex items-start justify-between">
        <h1 class="text-xl font-bold text-neutral-900 dark:text-neutral-100">AI settings</h1>
        <NuxtLink
          to="/app"
          class="text-sm text-neutral-400 hover:text-neutral-600 dark:text-neutral-500 dark:hover:text-neutral-300"
        >
          Back
        </NuxtLink>
      </div>
      <p class="mt-2 text-sm text-neutral-500 dark:text-neutral-400">
        Bring your own key. Point this at any OpenAI-compatible endpoint to enable the AI actions in
        the editor's <span class="font-medium">/</span> menu.
      </p>

      <p v-if="loading" class="mt-5 text-sm text-neutral-500 dark:text-neutral-400">Loading…</p>

      <form v-else class="mt-5 space-y-4" @submit.prevent="save">
        <div>
          <label
            for="ai-api-url"
            class="text-sm font-semibold text-neutral-700 dark:text-neutral-300"
          >
            API URL
          </label>
          <input
            id="ai-api-url"
            v-model="apiUrl"
            type="text"
            required
            placeholder="https://api.openai.com/v1"
            class="mt-2 w-full rounded border border-neutral-300 px-3 py-2 text-sm dark:border-neutral-700 dark:bg-neutral-800 dark:text-neutral-100"
          />
          <p class="mt-1 text-xs text-neutral-500 dark:text-neutral-400">
            Base URL only — <code>/chat/completions</code> is appended.
          </p>
        </div>

        <div>
          <label for="ai-api-key" class="text-sm font-semibold text-neutral-700 dark:text-neutral-300">
            API key
          </label>
          <input
            id="ai-api-key"
            v-model="apiKey"
            type="password"
            autocomplete="off"
            :placeholder="hasKey ? 'Leave blank to keep current key' : 'sk-…'"
            :required="!hasKey"
            class="mt-2 w-full rounded border border-neutral-300 px-3 py-2 text-sm dark:border-neutral-700 dark:bg-neutral-800 dark:text-neutral-100"
          />
          <p class="mt-1 text-xs" :class="hasKey ? 'text-green-600 dark:text-green-400' : 'text-neutral-500 dark:text-neutral-400'">
            {{ hasKey ? 'A key is currently set. Typing a new one replaces it.' : 'No key set yet.' }}
          </p>
        </div>

        <div>
          <label for="ai-model" class="text-sm font-semibold text-neutral-700 dark:text-neutral-300">
            Model
          </label>
          <input
            id="ai-model"
            v-model="model"
            type="text"
            required
            placeholder="gpt-4o-mini"
            class="mt-2 w-full rounded border border-neutral-300 px-3 py-2 text-sm dark:border-neutral-700 dark:bg-neutral-800 dark:text-neutral-100"
          />
        </div>

        <button
          type="submit"
          :disabled="saving"
          class="rounded bg-teal-600 px-3 py-2 text-sm font-medium text-white hover:bg-teal-700 disabled:opacity-50 dark:bg-teal-500 dark:text-neutral-950 dark:hover:bg-teal-400"
        >
          {{ saving ? '…' : 'Save' }}
        </button>
      </form>

      <p
        v-if="message"
        class="mt-3 text-sm"
        :class="message.ok ? 'text-green-600 dark:text-green-400' : 'text-red-600 dark:text-red-400'"
      >
        {{ message.text }}
      </p>
    </div>
  </div>
</template>
