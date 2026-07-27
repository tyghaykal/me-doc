import type * as Y from 'yjs'
import { type Ref, watch, onBeforeUnmount } from 'vue'

/**
 * Two-way bind a `Y.Text` to a Vue `ref<string>` using minimal diffs, so
 * concurrent edits merge through the CRDT instead of clobbering each other with
 * replace-all writes. Seeds the ref from the current text.
 *
 * ponytail: a remote edit resets the local caret to the value end (no cursor
 * transform). Fine for a Mermaid code pane; swap in y-codemirror if precise
 * multi-cursor editing becomes a requirement.
 */
export function bindYText(ytext: Y.Text, model: Ref<string>) {
  let applyingRemote = false
  model.value = ytext.toString()

  const observer = () => {
    const next = ytext.toString()
    if (next === model.value) return
    applyingRemote = true
    model.value = next
    applyingRemote = false
  }
  ytext.observe(observer)

  const stop = watch(model, (next) => {
    if (applyingRemote) return
    const prev = ytext.toString()
    if (next === prev) return

    // Longest common prefix and suffix → one delete + one insert.
    let start = 0
    const min = Math.min(prev.length, next.length)
    while (start < min && prev[start] === next[start]) start++
    let endPrev = prev.length
    let endNext = next.length
    while (endPrev > start && endNext > start && prev[endPrev - 1] === next[endNext - 1]) {
      endPrev--
      endNext--
    }
    ytext.doc?.transact(() => {
      if (endPrev > start) ytext.delete(start, endPrev - start)
      if (endNext > start) ytext.insert(start, next.slice(start, endNext))
    })
  })

  onBeforeUnmount(() => {
    ytext.unobserve(observer)
    stop()
  })
}
