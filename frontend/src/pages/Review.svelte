<script lang="ts">
  import { onMount } from 'svelte';
  import { cardsApi } from '../lib/api/cards';
  import Icon from '../lib/components/Icon.svelte';
  import { formatInterval, formatTime } from '../lib/format';
  import type { DueCard, Rating } from '../lib/types';

  const BUTTONS: { rating: Rating; label: string; key: keyof DueCard['preview'] }[] = [
    { rating: 1, label: 'Again', key: 'again' },
    { rating: 2, label: 'Hard', key: 'hard' },
    { rating: 3, label: 'Good', key: 'good' },
    { rating: 4, label: 'Easy', key: 'easy' },
  ];

  let ready = $state<DueCard[]>([]); // can be shown now
  let waiting = $state<DueCard[]>([]); // in the 10-minute relearn step
  let revealed = $state(false);
  let reviewed = $state(0);
  let loading = $state(true);
  let busy = $state(false);
  let error = $state('');

  let shownAt = Date.now();
  let wakeTimer: ReturnType<typeof setTimeout> | undefined;

  const current = $derived(ready[0]);
  const left = $derived(ready.length + waiting.length);
  const total = $derived(reviewed + left);
  const nextWaitingAt = $derived(
    waiting.length ? Math.min(...waiting.map((c) => new Date(c.due).getTime())) : null,
  );

  /**
   * Cards scheduled in whole days can be reviewed any time on their due day.
   * A card you just pressed Again on waits for its real 10-minute step.
   */
  function isReady(card: DueCard, now: number): boolean {
    if (!card.last_review) return true;
    const due = new Date(card.due).getTime();
    const step = due - new Date(card.last_review).getTime();
    return step >= 86_400_000 || due <= now;
  }

  async function load() {
    clearTimeout(wakeTimer);
    loading = true;
    error = '';
    try {
      const queue = await cardsApi.due(200);
      const now = Date.now();
      ready = queue.cards.filter((c) => isReady(c, now));
      waiting = queue.cards.filter((c) => !isReady(c, now));
      revealed = false;
      shownAt = Date.now();
      // Nothing to show but relearning cards: reload when the first is due.
      if (!ready.length && nextWaitingAt !== null) {
        wakeTimer = setTimeout(load, Math.max(1000, nextWaitingAt - now + 500));
      }
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      loading = false;
    }
  }

  function reveal() {
    revealed = true;
  }

  async function rate(rating: Rating) {
    if (!current || !revealed || busy) return;
    busy = true;
    try {
      await cardsApi.review(current.id, rating, Date.now() - shownAt);
      reviewed += 1;
      ready = ready.slice(1);
      revealed = false;
      shownAt = Date.now();
      // Out of cards: ask the server again. This picks up relearning cards
      // and anything past the first 200.
      if (!ready.length) await load();
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      busy = false;
    }
  }

  function reviewWaitingNow() {
    clearTimeout(wakeTimer);
    ready = waiting;
    waiting = [];
    shownAt = Date.now();
  }

  // Keyboard: Space/Enter shows the answer, 1-4 rate.
  function onkeydown(event: KeyboardEvent) {
    const target = event.target as HTMLElement;
    if (['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName)) return;
    if (!current) return;
    if (!revealed && (event.key === ' ' || event.key === 'Enter')) {
      event.preventDefault();
      reveal();
    } else if (revealed && ['1', '2', '3', '4'].includes(event.key)) {
      event.preventDefault();
      rate(Number(event.key) as Rating);
    }
  }

  /** Splits the example so the studied word can be highlighted. */
  function highlight(sentence: string, word: string): { text: string; hit: boolean }[] {
    const escaped = word.trim().replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    if (!escaped) return [{ text: sentence, hit: false }];
    return sentence
      .split(new RegExp(`(${escaped})`, 'gi'))
      .map((text, i) => ({ text, hit: i % 2 === 1 }));
  }

  onMount(() => {
    load();
    return () => clearTimeout(wakeTimer);
  });
</script>

<svelte:window {onkeydown} />

<div class="screen">
  <header class="bar">
    <a class="back" href="#/"><Icon name="arrow-left" stroke={2} /> Today</a>
    <div class="progress">
      {#if total > 0 && total <= 30}
        <div class="segments" aria-hidden="true">
          {#each Array(total) as _, i (i)}
            <span class:done={i < reviewed} class:now={i === reviewed && !!current}></span>
          {/each}
        </div>
      {:else if total > 30}
        <div class="line" aria-hidden="true"><div style:width="{(reviewed / total) * 100}%"></div></div>
      {/if}
      <span class="counts">{reviewed} done · {left} left</span>
    </div>
    {#if current}
      <a class="button edit" href="#/words/{current.id}"><Icon name="writing" size={18} /> Edit card</a>
    {:else}
      <span></span>
    {/if}
  </header>

  <div class="stage">
    {#if error}<p class="error">{error}</p>{/if}

    {#if loading && !current}
      <p class="muted">Loading…</p>
    {:else if current}
      <section class="card">
        {#if current.tags.length}
          <div class="tags">
            {#each current.tags as t (t)}<span class="tag">{t}</span>{/each}
          </div>
        {/if}
        <h1 class="word">{current.word}</h1>
        {#if revealed}
          <div class="answer">
            {#if current.meaning}<p class="meaning">{current.meaning}</p>{/if}
            {#if current.example}
              <p class="example">
                {#each highlight(current.example, current.word) as part, i (i)}
                  {#if part.hit}<mark>{part.text}</mark>{:else}{part.text}{/if}
                {/each}
              </p>
            {/if}
            {#if current.source}<span class="source">{current.source}</span>{/if}
            {#if !current.meaning && !current.example}
              <p class="muted">No meaning saved yet. <a href="#/words/{current.id}">Add one</a></p>
            {/if}
          </div>
        {:else}
          <p class="muted">Say the meaning out loud, then check.</p>
        {/if}
      </section>

      {#if revealed}
        <div class="ratings">
          {#each BUTTONS as b (b.rating)}
            <button type="button" class="rate {b.key}" disabled={busy} onclick={() => rate(b.rating)}>
              <span class="interval">{formatInterval(current.preview[b.key])}</span>
              <span class="label">{b.label}</span>
              <kbd>{b.rating}</kbd>
            </button>
          {/each}
        </div>
      {:else}
        <button type="button" class="reveal light" onclick={reveal}>Show answer <kbd>Space</kbd></button>
      {/if}
    {:else if waiting.length && nextWaitingAt !== null}
      <section class="card">
        <h1 class="done-title">Back soon</h1>
        <p class="muted">
          {waiting.length} {waiting.length === 1 ? 'card is' : 'cards are'} in relearning. Next one at
          <strong>{formatTime(new Date(nextWaitingAt).toISOString())}</strong>. This page reloads by itself.
        </p>
        <button type="button" onclick={reviewWaitingNow}>Review them now</button>
      </section>
    {:else}
      <section class="card">
        <div class="tick"><Icon name="check" size={34} stroke={2.6} /></div>
        <h1 class="done-title">All done for today</h1>
        <p class="muted">
          {#if reviewed}{reviewed} {reviewed === 1 ? 'card' : 'cards'} reviewed. Today is marked on your calendar.
          {:else}Nothing is due right now.{/if}
        </p>
        <div class="row wrap center">
          <a class="button primary" href="#/">Back to Today</a>
          <a class="button" href="#/writing">Write something</a>
        </div>
      </section>
    {/if}
  </div>
</div>

<style>
  .screen {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
  }
  .bar {
    height: 76px;
    flex-shrink: 0;
    padding: 0 48px;
    border-bottom: 1px solid var(--surface-2);
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    align-items: center;
  }
  .back {
    justify-self: start;
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text-2);
    font-weight: 600;
    text-decoration: none;
  }
  .edit {
    justify-self: end;
  }
  .progress {
    display: grid;
    justify-items: center;
    gap: 8px;
  }
  .segments {
    display: flex;
    gap: 4px;
  }
  .segments span {
    width: 26px;
    height: 6px;
    border-radius: 3px;
    background: var(--border);
  }
  .segments span.done {
    background: var(--accent);
  }
  .segments span.now {
    background: var(--text);
  }
  .line {
    width: 320px;
    height: 6px;
    border-radius: 3px;
    background: var(--border);
    overflow: hidden;
  }
  .line div {
    height: 100%;
    background: var(--accent);
  }
  .counts {
    font-family: var(--mono);
    font-size: 12px;
    color: var(--muted);
  }

  .stage {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 24px;
    padding: 32px 16px 40px;
  }
  .card {
    width: min(820px, 100%);
    min-height: 440px;
    padding: 40px 56px;
    border-radius: 28px;
    background: var(--surface);
    border: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 20px;
    text-align: center;
  }
  .tags {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
    justify-content: center;
  }
  .tag {
    height: 24px;
    padding: 0 10px;
    border-radius: 12px;
    border: 1px solid var(--border-strong);
    font-family: var(--mono);
    font-size: 11px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--muted);
    display: inline-flex;
    align-items: center;
  }
  .word {
    margin: 0;
    font-size: clamp(3rem, 9vw, 6rem);
    letter-spacing: -0.045em;
    overflow-wrap: anywhere;
  }
  .answer {
    width: 100%;
    padding-top: 22px;
    border-top: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 14px;
  }
  .meaning {
    margin: 0;
    font-family: var(--display);
    font-weight: 500;
    font-size: 26px;
    line-height: 1.3;
  }
  .example {
    margin: 0;
    max-width: 620px;
    font-size: 19px;
    font-style: italic;
    line-height: 1.55;
    color: var(--text-2);
  }
  mark {
    padding: 0 5px;
    border-radius: 5px;
    background: rgba(198, 243, 107, 0.16);
    color: var(--text);
    font-style: normal;
    font-weight: 600;
  }
  .source {
    font-family: var(--mono);
    font-size: 12px;
    color: var(--faint);
  }

  .reveal {
    width: min(820px, 100%);
    min-height: 72px;
    border-radius: 20px;
    font-size: 18px;
  }
  .reveal kbd {
    height: 26px;
    padding: 0 8px;
    border-radius: 7px;
    background: rgba(14, 15, 19, 0.1);
    display: inline-flex;
    align-items: center;
    font-size: 12px;
  }
  .ratings {
    width: min(820px, 100%);
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 12px;
  }
  .rate {
    min-height: 88px;
    border-radius: 18px;
    border-width: 1.5px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .interval {
    font-family: var(--mono);
    font-size: 12px;
    font-weight: 500;
    opacity: 0.85;
  }
  .label {
    font-family: var(--display);
    font-weight: 700;
    font-size: 20px;
  }
  .rate kbd {
    font-size: 11px;
    opacity: 0.7;
  }
  .rate.again {
    border-color: var(--again);
    background: var(--again-soft);
    color: var(--again);
  }
  .rate.hard {
    border-color: #3a3e49;
    background: var(--surface);
  }
  .rate.good {
    border-color: var(--accent);
    background: var(--accent);
    color: var(--on-accent);
  }
  .rate.good .label {
    font-weight: 800;
  }
  .rate.easy {
    border-color: var(--easy);
    background: var(--easy-soft);
    color: var(--easy);
  }
  .rate.again:hover:not(:disabled) {
    background: rgba(255, 155, 106, 0.2);
    color: var(--again);
  }
  .rate.good:hover:not(:disabled) {
    background: var(--accent-hover);
    color: var(--on-accent);
  }
  .rate.easy:hover:not(:disabled) {
    background: rgba(140, 200, 255, 0.16);
    color: var(--easy);
  }

  .tick {
    width: 72px;
    height: 72px;
    border-radius: 36px;
    background: var(--accent);
    color: var(--on-accent);
    display: grid;
    place-items: center;
  }
  .done-title {
    margin: 0;
    font-size: clamp(2.2rem, 6vw, 3.5rem);
  }
  .center {
    justify-content: center;
  }

  @media (max-width: 700px) {
    .bar {
      padding: 0 12px;
      grid-template-columns: auto 1fr auto;
      gap: 10px;
    }
    .segments span {
      width: auto;
      flex: 1;
    }
    .segments,
    .line {
      width: 100%;
    }
    .edit {
      padding: 0 10px;
      font-size: 0;
      gap: 0;
    }
    .card {
      min-height: 0;
      flex: 1;
      padding: 28px 22px;
    }
    .stage {
      justify-content: flex-start;
      padding: 16px 16px 28px;
    }
    .ratings {
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 10px;
    }
    .rate {
      min-height: 68px;
    }
    .rate kbd,
    .reveal kbd {
      display: none;
    }
  }
</style>
