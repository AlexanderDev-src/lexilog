<script lang="ts">
  import { onMount } from 'svelte';
  import { mistakesApi } from '../api/mistakes';
  import type { MistakeCount } from '../types';

  // Recurring mistakes tagged on this piece, with a count for each.
  // Every change is saved automatically half a second later.
  let { pieceId }: { pieceId: number } = $props();

  let mistakes = $state<MistakeCount[]>([]);
  let known = $state<string[]>([]);
  let draft = $state('');
  let saveState = $state<'idle' | 'saving' | 'saved' | 'error'>('idle');
  let error = $state('');
  let saveTimer: ReturnType<typeof setTimeout> | undefined;

  const suggestions = $derived(known.filter((k) => !mistakes.some((m) => m.tag === k)));

  async function load() {
    try {
      const [list, tags] = await Promise.all([mistakesApi.forPiece(pieceId), mistakesApi.tags()]);
      mistakes = list;
      known = tags.map((t) => t.name);
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    }
  }

  // onMount must stay synchronous for its cleanup function to be used.
  onMount(() => {
    load();
    // Save anything pending when the editor closes.
    return () => {
      if (saveState === 'saving') save();
    };
  });

  /** Same rule as the backend: single spaces, lower case. */
  function normalize(name: string): string {
    return name.trim().replace(/\s+/g, ' ').toLowerCase();
  }

  function update(next: MistakeCount[]) {
    mistakes = next.filter((m) => m.count > 0);
    saveState = 'saving';
    clearTimeout(saveTimer);
    saveTimer = setTimeout(save, 500);
  }

  function bump(tag: string, delta: number) {
    update(mistakes.map((m) => (m.tag === tag ? { ...m, count: Math.min(99, m.count + delta) } : m)));
  }

  function add(event: SubmitEvent) {
    event.preventDefault();
    const tag = normalize(draft);
    if (!tag) return;
    const exists = mistakes.some((m) => m.tag === tag);
    update(exists ? mistakes.map((m) => (m.tag === tag ? { ...m, count: m.count + 1 } : m)) : [...mistakes, { tag, count: 1 }]);
    draft = '';
  }

  /**
   * Called by the AI panel. For each suggested tag, keeps the HIGHER of the
   * current and suggested count, so asking the AI twice never double-counts.
   * (`export function` makes this callable from the parent via bind:this.)
   */
  export function merge(suggested: MistakeCount[]) {
    const next = [...mistakes];
    for (const s of suggested) {
      const tag = normalize(s.tag);
      const existing = next.find((m) => m.tag === tag);
      if (existing) existing.count = Math.max(existing.count, s.count);
      else next.push({ tag, count: s.count });
    }
    update(next);
  }

  async function save() {
    clearTimeout(saveTimer);
    try {
      await mistakesApi.replace(pieceId, mistakes);
      for (const m of mistakes) if (!known.includes(m.tag)) known = [...known, m.tag];
      saveState = 'saved';
    } catch (err) {
      saveState = 'error';
      error = err instanceof Error ? err.message : String(err);
    }
  }
</script>

<div class="stack">
  {#if mistakes.length}
    <ul class="mistakes">
      {#each mistakes as m (m.tag)}
        <li>
          <span class="name">{m.tag}</span>
          <button type="button" class="ghost step" aria-label="One fewer {m.tag}" onclick={() => bump(m.tag, -1)}>−</button>
          <span class="count">×{m.count}</span>
          <button type="button" class="ghost step" aria-label="One more {m.tag}" onclick={() => bump(m.tag, 1)}>+</button>
        </li>
      {/each}
    </ul>
  {:else}
    <p class="muted small">No mistakes tagged yet.</p>
  {/if}

  <form class="row" onsubmit={add}>
    <input list="mistake-tag-list" bind:value={draft} placeholder="Tag a mistake…" aria-label="Mistake tag" />
    <button type="submit">Add</button>
  </form>
  <datalist id="mistake-tag-list">
    {#each suggestions as s (s)}<option value={s}></option>{/each}
  </datalist>

  <span class="muted small">
    {#if saveState === 'saving'}Saving…{:else if saveState === 'saved'}Saved{/if}
    {#if saveState === 'error'}<span class="error">Not saved: {error}</span>{/if}
  </span>
</div>

<style>
  .mistakes {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 6px;
  }
  .mistakes li {
    display: flex;
    align-items: center;
    gap: 4px;
    min-height: 44px;
    padding: 0 4px 0 12px;
    border-radius: 10px;
    background: var(--surface-2);
    font-size: 14px;
  }
  .name {
    flex: 1;
  }
  .step {
    width: 32px;
    min-height: 32px;
    height: 32px;
    padding: 0;
    border-radius: 8px;
    color: var(--muted);
    font-size: 1.1rem;
  }
  .count {
    min-width: 1.8rem;
    text-align: center;
    font-family: var(--mono);
    font-size: 13px;
    font-weight: 600;
    color: var(--again);
  }
  form input {
    flex: 1;
    border-style: dashed;
  }
</style>
