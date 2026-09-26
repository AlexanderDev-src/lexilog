<script lang="ts">
  import { diffWords } from 'diff';

  interface Props {
    before: string;
    after: string;
    beforeLabel: string;
    afterLabel: string;
    /** Small grey text next to each label, e.g. "187 words · 23 min". */
    beforeMeta?: string;
    afterMeta?: string;
  }

  let { before, after, beforeLabel, afterLabel, beforeMeta = '', afterMeta = '' }: Props = $props();

  // A list of pieces: unchanged, removed (only in `before`) or added (only in `after`).
  const parts = $derived(diffWords(before, after));
  const removedWords = $derived(count(parts.filter((p) => p.removed)));
  const addedWords = $derived(count(parts.filter((p) => p.added)));

  function count(list: { value: string }[]): number {
    return list.reduce((sum, p) => sum + p.value.split(/\s+/).filter(Boolean).length, 0);
  }
</script>

<div class="row wrap stats">
  <span class="pill removed-pill">−{removedWords} words</span>
  <span class="pill added-pill">+{addedWords} words</span>
  <span class="muted small">Removed words are struck through on the left; new words are highlighted on the right.</span>
</div>
<div class="diff">
  <section>
    <header><h3>{beforeLabel}</h3><span>{beforeMeta}</span></header>
    <div class="text">
      {#each parts as part, i (i)}
        {#if !part.added}<span class:removed={part.removed}>{part.value}</span>{/if}
      {/each}
    </div>
  </section>
  <section>
    <header><h3>{afterLabel}</h3><span>{afterMeta}</span></header>
    <div class="text">
      {#each parts as part, i (i)}
        {#if !part.removed}<span class:added={part.added}>{part.value}</span>{/if}
      {/each}
    </div>
  </section>
</div>

<style>
  .stats {
    margin-bottom: 16px;
  }
  .pill {
    height: 30px;
    padding: 0 12px;
    border-radius: 15px;
    display: inline-flex;
    align-items: center;
    font-family: var(--mono);
    font-size: 13px;
    font-weight: 600;
  }
  .removed-pill {
    background: var(--again-soft);
    color: var(--again);
  }
  .added-pill {
    background: var(--success-soft);
    color: var(--accent);
  }
  .diff {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 20px;
  }
  @media (max-width: 800px) {
    .diff {
      grid-template-columns: minmax(0, 1fr);
    }
  }
  section {
    padding: 26px 30px;
    border-radius: var(--radius-lg);
    background: var(--surface);
    border: 1px solid var(--border);
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 12px;
    margin-bottom: 16px;
  }
  header h3 {
    font-size: 20px;
  }
  header span {
    font-family: var(--mono);
    font-size: 12px;
    color: var(--faint);
  }
  .text {
    white-space: pre-wrap;
    font-size: 17px;
    line-height: 1.85;
    color: var(--text-2);
  }
  .removed {
    padding: 1px 3px;
    border-radius: 4px;
    background: var(--again-soft);
    color: var(--again);
    text-decoration: line-through;
    text-decoration-color: rgba(255, 155, 106, 0.8);
  }
  .added {
    padding: 1px 3px;
    border-radius: 4px;
    background: var(--success-soft);
    color: var(--accent);
  }
</style>
