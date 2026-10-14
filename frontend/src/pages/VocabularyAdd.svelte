<script lang="ts">
  import { onMount } from 'svelte';
  import { cardsApi } from '../lib/api/cards';
  import CardForm from '../lib/components/CardForm.svelte';
  import Icon from '../lib/components/Icon.svelte';
  import { posLabel } from '../lib/format';
  import type { Card, CardInput, TagCount } from '../lib/types';

  let tags = $state<TagCount[]>([]);
  // Words added since this page opened, newest first.
  let added = $state<Card[]>([]);

  async function loadTags() {
    tags = await cardsApi.tags();
  }

  onMount(loadTags);

  async function addCard(input: CardInput) {
    const card = await cardsApi.create(input);
    added = [card, ...added];
    await loadTags();
  }
</script>

<a class="back" href="#/words"><Icon name="arrow-left" size={16} stroke={2.2} /> Vocabulary</a>

<header class="page-head">
  <span class="eyebrow">Your deck</span>
  <h1>Add a word</h1>
</header>

<div class="layout">
  <section class="panel form-panel">
    <p class="muted small intro">Only the word is required.</p>
    <CardForm
      submitLabel="Add word"
      keepContext
      autofocus
      tagSuggestions={tags.map((t) => t.name)}
      onsave={addCard}
    />
    <p class="small muted hint">Source and topics stay filled in for your next word from the same article.</p>
  </section>

  <section class="panel added-panel">
    <h2>Just added</h2>
    {#if added.length}
      <ul class="added">
        {#each added as card (card.id)}
          <li>
            <a href="#/words/{card.id}">
              <span class="word"
                >{card.word}{#if card.part_of_speech}<span class="pos">{posLabel(card.part_of_speech)}</span>{/if}</span
              >
              {#if card.meaning}<span class="meaning">{card.meaning}</span>{/if}
            </a>
          </li>
        {/each}
      </ul>
    {:else}
      <p class="muted small">Words you add here show up in this list.</p>
    {/if}
    <a class="all" href="#/words">All words →</a>
  </section>
</div>

<style>
  .back {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-weight: 600;
    font-size: 14px;
    text-decoration: none;
  }
  .page-head {
    margin: 16px 0 24px;
  }
  .page-head h1 {
    margin: 8px 0 0;
  }

  .layout {
    display: grid;
    grid-template-columns: repeat(12, minmax(0, 1fr));
    gap: 20px;
    align-items: start;
  }
  .form-panel {
    grid-column: span 7;
  }
  .added-panel {
    grid-column: span 5;
  }
  @media (max-width: 1100px) {
    .form-panel,
    .added-panel {
      grid-column: 1 / -1;
    }
  }
  .intro {
    margin: 0 0 18px;
  }
  .hint {
    margin: 12px 0 0;
  }

  .added {
    list-style: none;
    margin: 0 0 12px;
    padding: 0;
  }
  .added li {
    border-top: 1px solid var(--line);
  }
  .added a {
    display: grid;
    gap: 4px;
    padding: 12px 4px;
    color: var(--text);
    text-decoration: none;
  }
  .added a:hover .word {
    color: var(--accent);
  }
  .word {
    font-family: var(--display);
    font-weight: 700;
    font-size: 19px;
    letter-spacing: -0.015em;
  }
  .pos {
    margin-left: 8px;
    font-family: var(--sans);
    font-weight: 500;
    font-size: 14px;
    font-style: italic;
    letter-spacing: 0;
    color: var(--muted);
  }
  .meaning {
    font-size: 14px;
    color: var(--text-2);
  }
  .all {
    display: inline-block;
    margin-top: 4px;
    font-weight: 600;
    font-size: 14px;
    text-decoration: none;
  }
</style>
