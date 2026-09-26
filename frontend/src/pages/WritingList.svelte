<script lang="ts">
  import { onMount } from 'svelte';
  import { writingApi } from '../lib/api/writing';
  import Icon from '../lib/components/Icon.svelte';
  import { formatDate, PRESETS } from '../lib/format';
  import { navigate } from '../lib/router.svelte';
  import type { PieceSummary, WritingKind } from '../lib/types';

  let pieces = $state<PieceSummary[] | null>(null);
  let error = $state('');
  let creating = $state(false);

  onMount(async () => {
    try {
      pieces = await writingApi.list();
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    }
  });

  async function start(kind: WritingKind) {
    creating = true;
    try {
      const piece = await writingApi.create(kind);
      navigate(`/writing/${piece.id}`);
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
      creating = false;
    }
  }

  const KINDS: WritingKind[] = ['task1', 'task2', 'paragraph'];
</script>

<header class="page-head">
  <span class="eyebrow">Task 1 · Task 2 · free paragraphs</span>
  <h1>Writing</h1>
</header>

<div class="presets">
  {#each KINDS as kind (kind)}
    {@const preset = PRESETS[kind]}
    <button type="button" class="preset" disabled={creating} onclick={() => start(kind)}>
      <span class="plus"><Icon name="plus" size={18} stroke={2.2} /></span>
      <span class="preset-name">{preset.label}</span>
      <span class="preset-meta">
        {preset.minutes ? `${preset.minutes} min · ${preset.minWords} words` : 'free · no timer limit'}
      </span>
    </button>
  {/each}
</div>

{#if error}<p class="error">{error}</p>{/if}

<section class="panel">
  <h2>All pieces</h2>
  {#if pieces === null}
    <p class="muted">Loading…</p>
  {:else if pieces.length === 0}
    <p class="muted">Nothing yet. Pick a task above to start.</p>
  {:else}
    <ul class="pieces">
      {#each pieces as piece (piece.id)}
        {@const target = PRESETS[piece.kind].minWords}
        <li>
          <a href="#/writing/{piece.id}">
            <span class="badge">{PRESETS[piece.kind].label}</span>
            <span class="prompt">{piece.prompt || 'Untitled'}</span>
            <span class="meta">
              {formatDate(piece.written_on)}
              <span class:short={target !== null && piece.latest_word_count < target}>
                {piece.latest_word_count}{target ? `/${target}` : ''} words
              </span>
              {#if piece.version_count > 1}<span>v{piece.version_count}</span>{/if}
            </span>
          </a>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .page-head {
    margin-bottom: 24px;
  }
  .page-head h1 {
    margin: 8px 0 0;
  }
  .presets {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 16px;
    margin-bottom: 20px;
  }
  @media (max-width: 700px) {
    .presets {
      grid-template-columns: minmax(0, 1fr);
    }
  }
  .preset {
    min-height: 120px;
    padding: 20px 22px;
    border-radius: var(--radius-lg);
    border-color: var(--border);
    background: var(--surface);
    display: grid;
    grid-template-columns: 1fr auto;
    align-content: space-between;
    justify-items: start;
    text-align: left;
  }
  .preset:hover:not(:disabled) {
    border-color: var(--accent);
    background: var(--surface);
  }
  .plus {
    grid-column: 2;
    grid-row: 1;
    width: 34px;
    height: 34px;
    border-radius: 10px;
    background: var(--accent);
    color: var(--on-accent);
    display: grid;
    place-items: center;
  }
  .preset-name {
    grid-column: 1;
    grid-row: 1;
    font-family: var(--display);
    font-weight: 800;
    font-size: 26px;
    letter-spacing: -0.02em;
  }
  .preset-meta {
    grid-column: 1 / -1;
    font-family: var(--mono);
    font-size: 12px;
    font-weight: 500;
    color: var(--faint);
  }
  .pieces {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .pieces li + li {
    border-top: 1px solid var(--line);
  }
  .pieces a {
    display: grid;
    grid-template-columns: 96px minmax(0, 1fr) auto;
    align-items: center;
    gap: 16px;
    padding: 14px 4px;
    color: var(--text);
    text-decoration: none;
  }
  .pieces a:hover .prompt {
    color: var(--accent);
  }
  .prompt {
    font-family: var(--display);
    font-weight: 500;
    font-size: 17px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meta {
    display: flex;
    gap: 12px;
    font-family: var(--mono);
    font-size: 12px;
    color: var(--faint);
  }
  .meta .short {
    color: var(--amber);
  }
  @media (max-width: 700px) {
    .pieces a {
      grid-template-columns: auto minmax(0, 1fr);
    }
    .meta {
      grid-column: 1 / -1;
    }
  }
</style>
