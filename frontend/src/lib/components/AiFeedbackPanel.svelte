<script lang="ts">
  import { onMount } from 'svelte';
  import { aiApi } from '../api/ai';
  import { ApiError } from '../api/client';
  import { formatTime, formatTokens } from '../format';
  import type { AiFeedbackRecord, AiStatus, MistakeCount, Version } from '../types';

  interface Props {
    pieceId: number;
    /** The version open in the editor. */
    version: Version;
    /** Live word count of the editor. */
    words: number;
    /** Saves the editor's text, so the model sees the latest version. */
    beforeReview: () => Promise<void>;
    /** Selects the quoted words in the editor. */
    onselectquote: (quote: string) => void;
    /** Adds tags to the piece's mistake log. */
    onapplytags: (counts: MistakeCount[]) => void;
  }

  let { pieceId, version, words, beforeReview, onselectquote, onapplytags }: Props = $props();

  const MIN_WORDS = 30;

  let status = $state<AiStatus | null>(null);
  let records = $state<AiFeedbackRecord[]>([]);
  let model = $state('');
  let running = $state(false);
  let error = $state('');
  let rateLimited = $state(false);
  let shownId = $state<number | null>(null);
  let skipped = $state<string[]>([]);
  let applied = $state(false);

  const forVersion = $derived(records.filter((r) => r.version_id === version.id));
  const shown = $derived(forVersion.find((r) => r.id === shownId) ?? forVersion[0] ?? null);

  // Suggested tags from the shown review: each tag with how often it appears.
  const suggested = $derived.by(() => {
    const counts = new Map<string, number>();
    for (const issue of shown?.feedback.issues ?? []) {
      if (issue.tag) counts.set(issue.tag, (counts.get(issue.tag) ?? 0) + 1);
    }
    return [...counts].map(([tag, count]) => ({ tag, count }));
  });

  // After a 429 on one model, offer another that still has quota left.
  const fallback = $derived(status?.models.find((m) => m.id !== model && m.used_today < m.daily_limit) ?? null);

  onMount(async () => {
    try {
      const [s, list] = await Promise.all([aiApi.status(), aiApi.forPiece(pieceId)]);
      status = s;
      records = list;
      model = s.default_model ?? s.models[0]?.id ?? '';
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    }
  });

  async function run(chosen: string) {
    model = chosen;
    running = true;
    error = '';
    rateLimited = false;
    try {
      await beforeReview();
      const record = await aiApi.review(version.id, chosen);
      records = [record, ...records];
      shownId = record.id;
      skipped = [];
      applied = false;
    } catch (err) {
      rateLimited = err instanceof ApiError && err.status === 429;
      error = err instanceof Error ? err.message : String(err);
    } finally {
      running = false;
      status = await aiApi.status().catch(() => status);
    }
  }

  function applyTags() {
    onapplytags(suggested.filter((s) => !skipped.includes(s.tag)));
    applied = true;
  }

  function toggleSkip(tag: string) {
    skipped = skipped.includes(tag) ? skipped.filter((t) => t !== tag) : [...skipped, tag];
  }

  const BANDS: { key: keyof AiFeedbackRecord['feedback']['bands']; label: string }[] = [
    { key: 'task', label: 'Task' },
    { key: 'coherence', label: 'Coherence' },
    { key: 'lexical', label: 'Lexical' },
    { key: 'grammar', label: 'Grammar' },
    { key: 'overall', label: 'Overall' },
  ];
</script>

{#if status && !status.enabled}
  <p class="muted small">AI feedback is off. Add <code>AI_API_KEY</code> to the <code>.env</code> file to switch it on.</p>
{:else if status}
  <div class="row wrap controls">
    <label class="inline">
      <span class="visually-hidden">Model</span>
      <select bind:value={model} disabled={running}>
        {#each status.models as m (m.id)}
          <option value={m.id}>{m.id} · {formatTokens(m.used_today)} / {formatTokens(m.daily_limit)} today</option>
        {/each}
      </select>
    </label>
    <button type="button" class="primary" disabled={running || words < MIN_WORDS} onclick={() => run(model)}>
      {running ? 'Checking…' : shown ? 'Check again' : 'Get AI feedback'}
    </button>
  </div>
  {#if running}
    <p class="muted small">The model is reading v{version.version_no}. This takes 10–30 seconds.</p>
  {:else if words < MIN_WORDS}
    <p class="muted small">Write at least {MIN_WORDS} words first.</p>
  {/if}
  {#if error}
    <p class="error">{error}</p>
    {#if rateLimited && fallback}
      <button type="button" onclick={() => run(fallback.id)}>Try {fallback.id} instead</button>
    {/if}
  {/if}

  {#if shown}
    {@const fb = shown.feedback}
    <div class="stack result">
      <div class="bands">
        {#each BANDS as b (b.key)}
          <div class="band" class:overall={b.key === 'overall'}>
            <span class="band-value">{fb.bands[b.key].toFixed(1)}</span>
            <span class="band-label">{b.label}</span>
          </div>
        {/each}
      </div>
      <p class="muted small">Estimated bands, not an official score.</p>
      <p class="summary">{fb.summary}</p>

      {#if fb.issues.length}
        <h3>Issues <span class="muted small">(click one to find it in your text)</span></h3>
        <ol class="issues">
          {#each fb.issues as issue, i (i)}
            <li>
              <button type="button" class="issue" onclick={() => onselectquote(issue.quote)}>
                <span class="quote">“{issue.quote}”</span>
                {#if issue.tag}<span class="chip small">{issue.tag}</span>{/if}
                <span class="hint">{issue.hint}</span>
              </button>
            </li>
          {/each}
        </ol>
      {/if}

      {#if fb.questions.length}
        <h3>Questions to think about</h3>
        <ul class="questions">
          {#each fb.questions as q, i (i)}<li>{q}</li>{/each}
        </ul>
      {/if}

      {#if suggested.length}
        <div class="suggest">
          <h3>Add to mistake log?</h3>
          <div class="row wrap">
            {#each suggested as s (s.tag)}
              <label class="chip check">
                <input type="checkbox" checked={!skipped.includes(s.tag)} onchange={() => toggleSkip(s.tag)} disabled={applied} />
                {s.tag} ×{s.count}
              </label>
            {/each}
          </div>
          <div class="row">
            <button type="button" onclick={applyTags} disabled={applied || suggested.length === skipped.length}>
              {applied ? 'Added' : 'Add selected'}
            </button>
            <span class="muted small">Keeps the higher count if a tag is already there.</span>
          </div>
        </div>
      {/if}

      <div class="row wrap meta muted small">
        <span>{shown.model} · {formatTime(shown.created_at)} · {formatTokens(shown.input_tokens + shown.output_tokens)} tokens</span>
        {#if forVersion.length > 1}
          <label class="inline">
            Earlier:
            <select
              value={shown.id}
              onchange={(e) => {
                shownId = Number(e.currentTarget.value);
                skipped = [];
                applied = false;
              }}
            >
              {#each forVersion as r (r.id)}
                <option value={r.id}>{formatTime(r.created_at)} · {r.model}</option>
              {/each}
            </select>
          </label>
        {/if}
      </div>
    </div>
  {/if}
{/if}

<style>
  .controls select {
    max-width: 22rem;
  }
  .inline {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
  }
  .result {
    margin-top: 8px;
  }
  .bands {
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    gap: 8px;
  }
  .band {
    display: grid;
    justify-items: center;
    padding: 12px 4px;
    border: 1px solid transparent;
    border-radius: var(--radius);
    background: var(--surface-2);
  }
  .band.overall {
    border-color: var(--accent);
    background: var(--accent-soft);
    color: var(--accent);
  }
  .band-value {
    font-family: var(--display);
    font-size: 1.7rem;
    font-weight: 800;
    letter-spacing: -0.02em;
    font-variant-numeric: tabular-nums;
  }
  .band-label {
    font-size: 0.75rem;
    color: var(--muted);
  }
  .summary {
    margin: 0;
  }
  h3 {
    font-size: 0.95rem;
    margin: 8px 0 0;
  }
  .issues {
    margin: 0;
    padding: 0;
    list-style: none;
    display: grid;
    gap: 6px;
  }
  .issue {
    width: 100%;
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 4px 8px;
    justify-items: start;
    text-align: left;
    padding: 12px 14px;
    border-color: transparent;
    background: var(--surface-2);
    font-weight: 400;
  }
  .issue:hover:not(:disabled) {
    border-color: var(--border-strong);
    background: var(--surface-2);
  }
  .quote {
    font-style: italic;
    color: var(--again);
  }
  .issue .chip {
    justify-self: end;
  }
  .hint {
    grid-column: 1 / -1;
    color: var(--muted);
  }
  .questions {
    margin: 0;
    padding-left: 1.2rem;
  }
  .suggest {
    display: grid;
    gap: 10px;
    padding: 14px 16px;
    border: 1px dashed var(--border-strong);
    border-radius: 14px;
  }
  .check {
    cursor: pointer;
  }
  .check input {
    width: auto;
  }
  .meta {
    justify-content: space-between;
  }
  @media (max-width: 560px) {
    .bands {
      grid-template-columns: repeat(3, 1fr);
    }
  }
</style>
