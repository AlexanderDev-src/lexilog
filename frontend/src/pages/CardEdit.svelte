<script lang="ts">
  import { onMount } from 'svelte';
  import { cardsApi } from '../lib/api/cards';
  import CardForm from '../lib/components/CardForm.svelte';
  import Icon from '../lib/components/Icon.svelte';
  import { formatDue } from '../lib/format';
  import { navigate } from '../lib/router.svelte';
  import type { Card, CardInput } from '../lib/types';

  let { id }: { id: string } = $props();

  let card = $state<Card | null>(null);
  let tagNames = $state<string[]>([]);
  let error = $state('');
  let saved = $state(false);

  onMount(async () => {
    try {
      const [loaded, tags] = await Promise.all([cardsApi.get(Number(id)), cardsApi.tags()]);
      card = loaded;
      tagNames = tags.map((t) => t.name);
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    }
  });

  async function save(input: CardInput) {
    if (!card) return;
    card = await cardsApi.update(card.id, input);
    saved = true;
    setTimeout(() => (saved = false), 2500);
  }

  async function remove() {
    if (!card || !confirm(`Delete “${card.word}” and its review history?`)) return;
    await cardsApi.remove(card.id);
    navigate('/words');
  }

  /** 0-5 bars from FSRS stability, same scale as the word list. */
  function level(stability: number | null): number {
    if (stability === null) return 0;
    return stability < 1 ? 1 : stability < 7 ? 2 : stability < 30 ? 3 : stability < 90 ? 4 : 5;
  }
</script>

<a class="back" href="#/words"><Icon name="arrow-left" size={16} stroke={2.2} /> Vocabulary</a>
{#if error}<p class="error">{error}</p>{/if}

{#if card}
  <h1>{card.word}</h1>

  <div class="layout">
    <section class="panel form">
      <h2>Edit card</h2>
      <CardForm initial={card} tagSuggestions={tagNames} onsave={save} />
      {#if saved}<p class="success small">Saved.</p>{/if}
    </section>

    <div class="side">
      <section class="panel">
        <h2>Memory</h2>
        {#if card.stability === null}
          <p class="muted">New card, not reviewed yet. It is due today.</p>
        {:else}
          {@const lv = level(card.stability)}
          <div class="meter" title="Memory strength {lv} of 5">
            {#each [0, 1, 2, 3, 4] as i (i)}<span class:on={i < lv} style:height="{14 + i * 6}px"></span>{/each}
          </div>
          <dl>
            <dt>Next review</dt>
            <dd>{formatDue(card.due)}</dd>
            <dt>Reviews</dt>
            <dd>{card.reps}</dd>
            <dt>Forgotten</dt>
            <dd>{card.lapses}×</dd>
            <dt>Stability</dt>
            <dd title="Days until the chance of remembering drops to 90%">{card.stability.toFixed(1)} days</dd>
            <dt>Difficulty</dt>
            <dd>{card.difficulty?.toFixed(1)} / 10</dd>
          </dl>
        {/if}
      </section>
      <button type="button" class="danger" onclick={remove}>Delete card</button>
    </div>
  </div>
{/if}

<style>
  .back {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-weight: 600;
    font-size: 14px;
    text-decoration: none;
  }
  .layout {
    display: grid;
    grid-template-columns: repeat(12, minmax(0, 1fr));
    gap: 20px;
    align-items: start;
  }
  .form {
    grid-column: span 7;
  }
  .side {
    grid-column: span 5;
    display: grid;
    gap: 0;
    justify-items: start;
  }
  .side .panel {
    width: 100%;
  }
  @media (max-width: 1000px) {
    .form,
    .side {
      grid-column: 1 / -1;
    }
  }
  .meter {
    display: flex;
    align-items: flex-end;
    gap: 4px;
    margin-bottom: 18px;
  }
  .meter span {
    width: 10px;
    border-radius: 3px;
    background: #2a2d36;
  }
  .meter span.on {
    background: var(--accent);
  }
  dl {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 8px 20px;
    margin: 0;
  }
  dt {
    color: var(--muted);
  }
  dd {
    margin: 0;
    font-family: var(--mono);
    font-size: 0.95rem;
  }
</style>
