<script lang="ts" module>
  import type { CardSort } from '../lib/api/cards';

  // Kept outside the component, so coming back from a card's page
  // lands on the same search, sort and page.
  const saved = { q: '', tag: '', sort: 'newest' as CardSort, page: 1 };
</script>

<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { cardsApi } from '../lib/api/cards';
  import Icon from '../lib/components/Icon.svelte';
  import { formatInterval, posLabel } from '../lib/format';
  import type { Card, TagCount } from '../lib/types';

  // 5 columns x 8 rows.
  const PAGE_SIZE = 40;

  const SORTS: { value: CardSort; label: string }[] = [
    { value: 'newest', label: 'Newest' },
    { value: 'az', label: 'A–Z' },
    { value: 'za', label: 'Z–A' },
    { value: 'pos', label: 'Part of speech' },
  ];

  let q = $state(saved.q); // what is typed
  let search = $state(saved.q); // what was searched, 200 ms later
  let tag = $state(saved.tag);
  let sort = $state<CardSort>(saved.sort);
  let page = $state(saved.page);

  let cards = $state<Card[]>([]);
  let total = $state(0);
  let tags = $state<TagCount[]>([]);
  let loading = $state(false);
  let error = $state('');
  let searchInput: HTMLInputElement | undefined = $state();
  let listTop: HTMLElement | undefined = $state();

  const pageCount = $derived(Math.max(1, Math.ceil(total / PAGE_SIZE)));
  const first = $derived(total === 0 ? 0 : (page - 1) * PAGE_SIZE + 1);
  const last = $derived(Math.min(page * PAGE_SIZE, total));

  $effect(() => {
    Object.assign(saved, { q: search, tag, sort, page });
  });

  // Search 200 ms after the user stops typing, from page 1.
  $effect(() => {
    const typed = q;
    if (typed === search) return;
    const timeout = setTimeout(() => {
      search = typed;
      page = 1;
    }, 200);
    return () => clearTimeout(timeout);
  });

  // Each load gets a number, so a slow older reply can't overwrite a newer one.
  let lastLoad = 0;
  $effect(() => {
    const filter = { q: search, tag };
    const wanted = { ...filter, sort, limit: PAGE_SIZE, offset: (page - 1) * PAGE_SIZE };
    const id = ++lastLoad;
    loading = true;
    error = '';
    Promise.all([cardsApi.list(wanted), cardsApi.count(filter)])
      .then(([found, count]) => {
        if (id !== lastLoad) return;
        total = count;
        // The page can run past the end after words were deleted.
        const lastPage = Math.max(1, Math.ceil(count / PAGE_SIZE));
        if (page > lastPage) page = lastPage;
        else cards = found;
      })
      .catch((err) => {
        if (id === lastLoad) error = err instanceof Error ? err.message : String(err);
      })
      .finally(() => {
        if (id === lastLoad) loading = false;
      });
  });

  onMount(async () => {
    tags = await cardsApi.tags();
  });

  function setTag(name: string) {
    tag = name;
    page = 1;
  }

  function setSort(value: CardSort) {
    sort = value;
    page = 1;
  }

  async function goTo(n: number) {
    page = Math.min(Math.max(1, n), pageCount);
    await tick();
    if (listTop && listTop.getBoundingClientRect().top < 0) listTop.scrollIntoView({ block: 'start' });
  }

  /** Page buttons: first, last, and two either side of the current one; gaps become "…". */
  function pageItems(current: number, count: number): (number | null)[] {
    const items: (number | null)[] = [];
    for (let n = 1; n <= count; n++) {
      if (n === 1 || n === count || Math.abs(n - current) <= 2) items.push(n);
      else if (items[items.length - 1] !== null) items.push(null);
    }
    return items;
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
  <div class="head-actions">
    <label class="search">
      <Icon name="search" size={18} stroke={2} />
      <span class="visually-hidden">Search words</span>
      <input bind:this={searchInput} type="search" bind:value={q} placeholder="Search words, meanings, examples" />
      <kbd class="key">/</kbd>
    </label>
    <a class="button primary" href="#/words/new"><Icon name="plus" size={18} stroke={2} /> Add word</a>
  </div>
</header>

<section class="panel" bind:this={listTop}>
  <div class="toolbar">
    <div class="sorts" role="group" aria-label="Sort words">
      <span class="eyebrow">Sort</span>
      {#each SORTS as s (s.value)}
        <button
          type="button"
          class="chip"
          class:active={sort === s.value}
          aria-pressed={sort === s.value}
          onclick={() => setSort(s.value)}>{s.label}</button
        >
      {/each}
    </div>
    <span class="muted small">
      {#if total}{first}–{last} of {total}{:else if !loading}0 words{/if}
    </span>
  </div>

  {#if tags.length}
    <div class="filters" role="group" aria-label="Filter by topic">
      <button type="button" class="chip" class:active={tag === ''} aria-pressed={tag === ''} onclick={() => setTag('')}>All</button>
      {#each tags as t (t.name)}
        <button
          type="button"
          class="chip"
          class:active={tag === t.name}
          aria-pressed={tag === t.name}
          onclick={() => setTag(tag === t.name ? '' : t.name)}
        >
          {t.name} <span class="count">{t.card_count}</span>
        </button>
      {/each}
    </div>
  {/if}

  {#if error}<p class="error">{error}</p>{/if}

  <ul class="words" class:loading>
    {#each cards as card (card.id)}
      {@const st = status(card)}
      {@const level = memoryLevel(card)}
      <li>
        <a href="#/words/{card.id}">
          <span class="word">{card.word}</span>
          {#if card.part_of_speech}<span class="pos">{posLabel(card.part_of_speech)}</span>{/if}
          {#if card.meaning}<span class="meaning">{card.meaning}</span>{/if}
          <span class="foot">
            <span class="meter" title="Memory strength {level} of 5">
              {#each [0, 1, 2, 3, 4] as i (i)}<span class:on={i < level} style:height="{8 + i * 3}px"></span>{/each}
            </span>
            <span class="status {st.kind}">{st.text}</span>
          </span>
        </a>
      </li>
    {:else}
      {#if !loading}
        <li class="empty muted">
          {#if search || tag}No matches.{:else}No words yet. <a href="#/words/new">Add your first one.</a>{/if}
        </li>
      {/if}
    {/each}
  </ul>

  {#if pageCount > 1}
    <nav class="pager" aria-label="Pages">
      <button type="button" class="ghost" onclick={() => goTo(page - 1)} disabled={page === 1} aria-label="Previous page">
        <Icon name="arrow-left" size={16} stroke={2.2} />
      </button>
      {#each pageItems(page, pageCount) as n, i (n ?? `gap-${i}`)}
        {#if n === null}
          <span class="gap muted">…</span>
        {:else}
          <button
            type="button"
            class="num"
            class:active={n === page}
            aria-current={n === page ? 'page' : undefined}
            onclick={() => goTo(n)}>{n}</button
          >
        {/if}
      {/each}
      <button type="button" class="ghost" onclick={() => goTo(page + 1)} disabled={page === pageCount} aria-label="Next page">
        <Icon name="arrow-right" size={16} stroke={2.2} />
      </button>
    </nav>
  {/if}
</section>

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
  .head-actions {
    display: flex;
    gap: 12px;
    flex-wrap: wrap;
    width: min(560px, 100%);
  }
  .head-actions .button {
    flex-shrink: 0;
  }
  .search {
    flex: 1;
    min-width: 220px;
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

  .toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
    margin-bottom: 12px;
  }
  .sorts,
  .filters {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
  }
  .sorts .eyebrow {
    margin-right: 6px;
  }
  .filters {
    padding-top: 12px;
    border-top: 1px solid var(--line);
    margin-bottom: 16px;
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
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    gap: 10px;
    transition: opacity 0.15s;
  }
  .words.loading {
    opacity: 0.6;
  }
  @media (max-width: 1250px) {
    .words {
      grid-template-columns: repeat(4, minmax(0, 1fr));
    }
  }
  @media (max-width: 1000px) {
    .words {
      grid-template-columns: repeat(3, minmax(0, 1fr));
    }
  }
  @media (max-width: 640px) {
    .words {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
  .words a {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 14px;
    border: 1px solid var(--line);
    border-radius: var(--radius);
    color: var(--text);
    text-decoration: none;
  }
  .words a:hover {
    border-color: var(--border-strong);
    background: var(--hover);
  }
  .words a:hover .word {
    color: var(--accent);
  }
  .word {
    font-family: var(--display);
    font-weight: 700;
    font-size: 19px;
    letter-spacing: -0.015em;
    overflow-wrap: anywhere;
  }
  .pos {
    font-size: 13px;
    font-style: italic;
    color: var(--muted);
  }
  .meaning {
    font-size: 14px;
    color: var(--text-2);
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .foot {
    margin-top: auto;
    padding-top: 8px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
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
    height: 22px;
    padding: 0 8px;
    border-radius: 11px;
    display: grid;
    place-items: center;
    font-family: var(--mono);
    font-size: 11px;
    font-weight: 600;
    white-space: nowrap;
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
  .empty {
    grid-column: 1 / -1;
    padding: 20px 4px;
  }

  .pager {
    display: flex;
    justify-content: center;
    align-items: center;
    gap: 4px;
    flex-wrap: wrap;
    margin-top: 20px;
    padding-top: 16px;
    border-top: 1px solid var(--line);
  }
  .pager button {
    min-width: 40px;
    min-height: 40px;
    padding: 0 10px;
  }
  .pager .num {
    border-color: transparent;
    font-family: var(--mono);
    font-size: 0.85rem;
  }
  .pager .num.active {
    border-color: var(--accent);
    background: var(--accent-soft);
    color: var(--accent);
  }
  .gap {
    min-width: 24px;
    text-align: center;
  }
</style>
