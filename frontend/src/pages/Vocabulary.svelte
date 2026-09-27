<script lang="ts">
  import { onMount } from 'svelte';
  import { cardsApi } from '../lib/api/cards';
  import CardForm from '../lib/components/CardForm.svelte';
  import Icon from '../lib/components/Icon.svelte';
  import { formatInterval, posLabel } from '../lib/format';
  import type { Card, CardInput, TagCount } from '../lib/types';

  const PAGE_SIZE = 50;

  let q = $state('');
  let tag = $state('');
  let cards = $state<Card[]>([]);
  let tags = $state<TagCount[]>([]);
  let hasMore = $state(false);
  let loading = $state(false);
  let error = $state('');
  let added = $state<string | null>(null);
  let searchInput: HTMLInputElement | undefined = $state();

  async function loadCards(reset: boolean) {
    loading = true;
    error = '';
    try {
      const offset = reset ? 0 : cards.length;
      const page = await cardsApi.list({ q, tag, limit: PAGE_SIZE, offset });
      cards = reset ? page : [...cards, ...page];
      hasMore = page.length === PAGE_SIZE;
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      loading = false;
    }
  }

  async function loadTags() {
    tags = await cardsApi.tags();
  }

  // Re-run the search 200 ms after the user stops typing or changes the tag.
  // $effect re-runs whenever a $state it reads (q, tag) changes.
  $effect(() => {
    void q;
    void tag;
    const timeout = setTimeout(() => loadCards(true), 200);
    return () => clearTimeout(timeout);
  });

  onMount(loadTags);

  async function addCard(input: CardInput) {
    const card = await cardsApi.create(input);
    added = card.word;
    setTimeout(() => (added = null), 3000);
    await Promise.all([loadCards(true), loadTags()]);
  }

  // "/" anywhere on the page (outside a text box) jumps to search.
  function onkeydown(event: KeyboardEvent) {
    const target = event.target as HTMLElement;
    if (event.key === '/' && !['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName)) {
      event.preventDefault();
      searchInput?.focus();
    }
  }

  // Local midnight tonight: anything due before it is "due today".
  const endOfToday = (() => {
    const d = new Date();
    d.setHours(24, 0, 0, 0);
    return d.getTime();
  })();

  function status(card: Card): { text: string; kind: 'fresh' | 'today' | 'later' } {
    if (card.stability === null) return { text: 'new', kind: 'fresh' };
    const due = new Date(card.due).getTime();
    if (due < endOfToday) return { text: 'due today', kind: 'today' };
    return { text: `in ${formatInterval((due - Date.now()) / 1000)}`, kind: 'later' };
  }

  /** 0-5 bars from FSRS stability (days until recall drops to 90%). */
  function memoryLevel(card: Card): number {
    const s = card.stability;
    if (s === null) return 0;
    if (s < 1) return 1;
    if (s < 7) return 2;
    if (s < 30) return 3;
    if (s < 90) return 4;
    return 5;
  }
</script>

<svelte:window {onkeydown} />

<header class="page-head">
  <div>
    <span class="eyebrow">Your deck</span>
    <h1>Vocabulary</h1>
  </div>
  <label class="search">
    <Icon name="search" size={18} stroke={2} />
    <span class="visually-hidden">Search words</span>
    <input bind:this={searchInput} type="search" bind:value={q} placeholder="Search words, meanings, examples" />
    <kbd class="key">/</kbd>
  </label>
</header>

<div class="layout">
  <section class="panel form-panel">
    <h2>Add a word</h2>
    <p class="muted small intro">Only the word is required.</p>
    <CardForm
      submitLabel="Add word"
      keepContext
      autofocus
      tagSuggestions={tags.map((t) => t.name)}
      onsave={addCard}
    />
    <p class="small hint">
      {#if added}<span class="success">Added “{added}”.</span>
      {:else}<span class="muted">Source and topics stay filled in for your next word from the same article.</span>{/if}
    </p>
  </section>

  <section class="panel list-panel">
    {#if tags.length}
      <div class="filters" role="group" aria-label="Filter by topic">
        <button type="button" class="chip" class:active={tag === ''} aria-pressed={tag === ''} onclick={() => (tag = '')}>All</button>
        {#each tags as t (t.name)}
          <button
            type="button"
            class="chip"
            class:active={tag === t.name}
            aria-pressed={tag === t.name}
            onclick={() => (tag = tag === t.name ? '' : t.name)}
          >
            {t.name} <span class="count">{t.card_count}</span>
          </button>
        {/each}
      </div>
    {/if}

    {#if error}<p class="error">{error}</p>{/if}

    <ul class="words">
      {#each cards as card (card.id)}
        {@const st = status(card)}
        {@const level = memoryLevel(card)}
        <li>
          <a href="#/words/{card.id}">
            <div class="top">
              <span class="word"
                >{card.word}{#if card.part_of_speech}<span class="pos">{posLabel(card.part_of_speech)}</span>{/if}</span
              >
              {#each card.tags as t (t)}<span class="chip small">{t}</span>{/each}
              <span class="right">
                <span class="meter" title="Memory strength {level} of 5">
                  {#each [0, 1, 2, 3, 4] as i (i)}<span class:on={i < level} style:height="{8 + i * 3}px"></span>{/each}
                </span>
                <span class="status {st.kind}">{st.text}</span>
              </span>
            </div>
            {#if card.meaning}<span class="meaning">{card.meaning}</span>{/if}
            {#if card.example}<span class="example">{card.example}</span>{/if}
          </a>
        </li>
      {:else}
        {#if !loading}<li class="empty muted">{q || tag ? 'No matches.' : 'No words yet. Add your first one.'}</li>{/if}
      {/each}
    </ul>

    <div class="list-foot">
      <span class="muted small">Showing {cards.length}</span>
      {#if hasMore}
        <button type="button" onclick={() => loadCards(false)} disabled={loading}>Load more</button>
      {/if}
    </div>
  </section>
</div>

<style>
  .page-head {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    gap: 16px;
    flex-wrap: wrap;
    margin-bottom: 24px;
  }
  .page-head h1 {
    margin: 8px 0 0;
  }
  .search {
    width: min(380px, 100%);
    height: 48px;
    padding: 0 8px 0 16px;
    border-radius: 14px;
    border: 1px solid var(--border-strong);
    background: var(--surface);
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 10px;
    color: var(--faint);
  }
  .search:focus-within {
    border-color: var(--accent);
    box-shadow: 0 0 0 4px var(--accent-soft);
  }
  .search input {
    flex: 1;
    min-width: 0;
    height: 100%;
    padding: 0;
    border: none;
    background: transparent;
    box-shadow: none;
  }

  .layout {
    display: grid;
    grid-template-columns: repeat(12, minmax(0, 1fr));
    gap: 20px;
    align-items: start;
  }
  .form-panel {
    grid-column: span 5;
  }
  .list-panel {
    grid-column: span 7;
  }
  @media (max-width: 1100px) {
    .form-panel,
    .list-panel {
      grid-column: 1 / -1;
    }
  }
  .form-panel h2 {
    margin-bottom: 4px;
  }
  .intro {
    margin: 0 0 18px;
  }
  .hint {
    margin: 12px 0 0;
  }

  .filters {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-bottom: 8px;
  }
  .count {
    font-family: var(--mono);
    color: var(--faint);
  }
  .chip.active .count {
    color: inherit;
    opacity: 0.8;
  }

  .words {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .words li {
    border-top: 1px solid var(--line);
  }
  .words a {
    display: grid;
    gap: 5px;
    padding: 14px 4px;
    color: var(--text);
    text-decoration: none;
  }
  .words a:hover .word {
    color: var(--accent);
  }
  .top {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }
  .word {
    font-family: var(--display);
    font-weight: 700;
    font-size: 22px;
    letter-spacing: -0.015em;
  }
  .pos {
    margin-left: 8px;
    font-family: var(--sans);
    font-weight: 500;
    font-size: 15px;
    font-style: italic;
    letter-spacing: 0;
    color: var(--muted);
  }
  .right {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .meter {
    display: flex;
    align-items: flex-end;
    gap: 2px;
  }
  .meter span {
    width: 4px;
    border-radius: 2px;
    background: #2a2d36;
  }
  .meter span.on {
    background: var(--accent);
  }
  .status {
    min-width: 76px;
    height: 24px;
    padding: 0 9px;
    border-radius: 12px;
    display: grid;
    place-items: center;
    font-family: var(--mono);
    font-size: 11px;
    font-weight: 600;
  }
  .status.fresh {
    background: var(--accent-soft);
    color: var(--accent);
  }
  .status.today {
    background: var(--again-soft);
    color: var(--again);
  }
  .status.later {
    background: var(--hover);
    color: var(--text-2);
  }
  .meaning {
    font-size: 15px;
    color: var(--text-2);
  }
  .example {
    font-size: 14px;
    font-style: italic;
    color: var(--faint);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .empty {
    padding: 20px 4px;
  }
  .list-foot {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding-top: 12px;
    border-top: 1px solid var(--line);
  }
</style>
