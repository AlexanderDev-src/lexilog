<script lang="ts">
  import { cardsApi } from '../api/cards';
  import type { DeckMatch, Matcher } from '../deckWords';
  import type { DeckWord } from '../types';

  // "Deck words used" in the essay, and a quick form to add a selected word
  // or phrase to the deck with its sentence as the example.
  interface Props {
    /** Deck words found in the essay, in text order. */
    matches: DeckMatch[];
    matcher: Matcher;
    /** The word or phrase the learner selected, with its sentence. */
    picked: { text: string; sentence: string } | null;
    /** Goes into the new card's Source field. */
    source: string;
    onadded: (word: DeckWord) => void;
    ondismiss: () => void;
  }

  let { matches, matcher, picked, source, onadded, ondismiss }: Props = $props();

  let word = $state('');
  let meaning = $state('');
  let example = $state('');
  let saving = $state(false);
  let error = $state('');
  let added = $state<DeckWord | null>(null);

  // A new selection refills the form.
  $effect(() => {
    if (picked) {
      word = picked.text;
      example = picked.sentence;
      meaning = '';
      error = '';
      added = null;
    }
  });

  // Each card once, with how often it appears.
  const used = $derived.by(() => {
    const byCard = new Map<number, { cardId: number; word: string; count: number }>();
    for (const m of matches) {
      const entry = byCard.get(m.cardId) ?? { cardId: m.cardId, word: m.word, count: 0 };
      entry.count++;
      byCard.set(m.cardId, entry);
    }
    return [...byCard.values()];
  });

  // Is the selected text (as a whole) already a card?
  const existing = $derived.by(() => {
    const text = word.trim();
    return matcher(text).find((m) => m.start === 0 && m.end === text.length) ?? null;
  });

  async function add(event: SubmitEvent) {
    event.preventDefault();
    if (!word.trim()) return;
    saving = true;
    error = '';
    try {
      const card = await cardsApi.create({ word, meaning, example, source, tags: [] });
      added = { id: card.id, word: card.word };
      onadded(added);
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      saving = false;
    }
  }
</script>

<section class="panel deck">
  <div class="row spread">
    <h2>Deck words used</h2>
    <span class="total">{used.length}</span>
  </div>
  {#if used.length}
    <div class="chips">
      {#each used as u (u.cardId)}
        <a class="chip small word" href="#/words/{u.cardId}" title="Open the card">{u.word}{u.count > 1 ? ` ×${u.count}` : ''}</a>
      {/each}
    </div>
  {/if}

  {#if picked}
    <form class="quick" onsubmit={add}>
      <div class="row spread">
        <h3>Add to deck</h3>
        <button type="button" class="ghost close" onclick={ondismiss} aria-label="Close">×</button>
      </div>
      <label>
        Word or phrase
        <input bind:value={word} autocomplete="off" spellcheck="false" />
      </label>
      {#if existing}
        <p class="small">
          Already in your deck as <a href="#/words/{existing.cardId}">{existing.word}</a>.
        </p>
      {:else}
        <label>
          Meaning <span class="muted">(optional, you can add it later)</span>
          <input bind:value={meaning} autocomplete="off" />
        </label>
        <label>
          Example
          <textarea bind:value={example} rows="3"></textarea>
        </label>
        <button type="submit" class="primary" disabled={saving || !word.trim()}>{saving ? 'Adding…' : 'Add to deck'}</button>
      {/if}
      {#if error}<p class="error small">{error}</p>{/if}
    </form>
  {:else if added}
    <p class="small success">Added <a href="#/words/{added.id}">{added.word}</a> to the deck.</p>
  {:else}
    <p class="muted small">
      {used.length ? 'Dotted underline in your text. ' : ''}Select a word or phrase in your essay or its feedback to add it to
      the deck.
    </p>
  {/if}
</section>

<style>
  .deck {
    display: grid;
    gap: 12px;
    padding: 20px 22px;
  }
  .deck h2 {
    margin: 0;
    font-size: 1.1rem;
  }
  .total {
    font-family: var(--mono);
    font-size: 13px;
    color: var(--accent);
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .word {
    border-color: transparent;
    background: var(--accent-soft);
    color: var(--accent);
    font-size: 13px;
    font-weight: 600;
    text-decoration: none;
  }
  .word:hover {
    color: var(--accent-hover);
  }
  .deck p {
    margin: 0;
  }
  .quick {
    display: grid;
    gap: 10px;
    padding: 14px 16px;
    border: 1px dashed var(--border-strong);
    border-radius: 14px;
  }
  .quick textarea {
    font-style: italic;
    line-height: 1.5;
    resize: none;
  }
  .close {
    width: 32px;
    min-height: 32px;
    padding: 0;
    font-size: 1.2rem;
    color: var(--muted);
  }
</style>
