<script lang="ts">
  import { onMount } from 'svelte';
  import { cardsApi } from '../lib/api/cards';
  import { dashboardApi } from '../lib/api/dashboard';
  import Heatmap from '../lib/components/Heatmap.svelte';
  import Icon from '../lib/components/Icon.svelte';
  import PracticeForm from '../lib/components/PracticeForm.svelte';
  import { formatDate, posLabel, PRESETS } from '../lib/format';
  import type { Card, Dashboard } from '../lib/types';

  let dashboard = $state<Dashboard | null>(null);
  let recent = $state<Card[]>([]);
  let error = $state('');

  async function load() {
    try {
      const [d, cards] = await Promise.all([dashboardApi.get(), cardsApi.list({ limit: 3 })]);
      dashboard = d;
      recent = cards;
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    }
  }

  onMount(load);

  function parse(iso: string): Date {
    const [y, m, d] = iso.split('-').map(Number);
    return new Date(Date.UTC(y, m - 1, d));
  }

  const dateLine = (iso: string) =>
    parse(iso).toLocaleDateString('en-GB', { weekday: 'long', day: 'numeric', month: 'long', timeZone: 'UTC' });
  const monthName = (iso: string) => parse(iso).toLocaleDateString('en-GB', { month: 'long', timeZone: 'UTC' });
</script>

<header class="page-head">
  <div>
    <span class="eyebrow">{dashboard ? dateLine(dashboard.today) : ' '}</span>
    <h1>Today</h1>
  </div>
  <div class="row">
    <a class="button" href="#/words"><Icon name="plus" size={18} stroke={2} /> Add word</a>
    <a class="button" href="#/writing"><Icon name="writing" size={18} /> New essay</a>
  </div>
</header>

{#if error}<p class="error">{error}</p>{/if}

{#if dashboard}
  {@const latest = dashboard.latest_piece}
  {@const next = dashboard.next_card}
  <div class="two">
    <section class="panel due">
      <div class="due-main">
        <span class="eyebrow">Due today</span>
        <div class="due-count">
          <span class="big" class:zero={dashboard.due_today === 0}>{dashboard.due_today}</span>
          <span class="big-label">{dashboard.due_today === 1 ? 'card' : 'cards'}<br />waiting</span>
        </div>
        {#if dashboard.due_today > 0}
          <div class="row wrap kinds">
            {#if dashboard.due.new}<span class="kind"><i class="dot new"></i>{dashboard.due.new} new</span>{/if}
            {#if dashboard.due.review}<span class="kind"><i class="dot review"></i>{dashboard.due.review} review</span>{/if}
            {#if dashboard.due.again}<span class="kind"><i class="dot again"></i>{dashboard.due.again} again</span>{/if}
          </div>
          <a class="button primary go" href="#/review">Start review <Icon name="arrow-right" stroke={2.2} /></a>
        {:else}
          <p class="muted small">All caught up. New words are due as soon as you add them.</p>
          <a class="button" href="#/words">Add words</a>
        {/if}
      </div>
      {#if next}
        <div class="deck" aria-hidden="true">
          <div class="deck-card back2"></div>
          <div class="deck-card back1"></div>
          <div class="deck-card front">
            <span class="eyebrow">{next.tags[0] ?? 'next up'}</span>
            <span class="deck-word">{next.word}</span>
          </div>
        </div>
      {/if}
    </section>

    <section class="panel latest">
      <div class="row spread">
        <span class="eyebrow">Latest writing</span>
        {#if latest}<a class="more" href="#/writing/{latest.id}">Continue →</a>{/if}
      </div>
      {#if latest}
        {@const target = PRESETS[latest.kind].minWords}
        <div class="row">
          <span class="badge">{PRESETS[latest.kind].label}</span>
          <span class="muted small">{formatDate(latest.written_on)} · version {latest.version_count}</span>
        </div>
        <p class="prompt">{latest.prompt || 'Untitled'}</p>
        <div class="progress-block">
          <div class="row spread small">
            <span class="mono">{latest.latest_word_count}{target ? ` / ${target}` : ''} words</span>
            {#if latest.latest_seconds_spent}
              <span class="muted">{Math.round(latest.latest_seconds_spent / 60)} min on the timer</span>
            {/if}
          </div>
          {#if target}
            <div class="bar"><div style:width="{Math.min(100, (latest.latest_word_count / target) * 100)}%"></div></div>
          {/if}
        </div>
      {:else}
        <p class="prompt muted">No writing yet.</p>
        <a class="button" href="#/writing">Start writing</a>
      {/if}
    </section>
  </div>

  <section class="panel">
    <div class="calendar-head">
      <div>
        <h2>Practice calendar</h2>
        <p class="muted small">Reviews, writing and logged sessions. Days off are fine: nothing here resets.</p>
      </div>
      <div class="stats">
        <div><span class="stat">{dashboard.active_days_year}</span><span class="eyebrow-ish">days in the last year</span></div>
        <div><span class="stat lime">{dashboard.active_days_month}</span><span class="eyebrow-ish">days in {monthName(dashboard.today)}</span></div>
      </div>
    </div>
    <Heatmap from={dashboard.from} today={dashboard.today} days={dashboard.days} />
  </section>

  <div class="two">
    <section class="panel" id="log">
      <h2>Log other practice</h2>
      <PracticeForm today={dashboard.today} showDate={false} onsaved={load} />
    </section>

    <section class="panel recent">
      <div class="row spread">
        <h2>Recently added</h2>
        <a class="more" href="#/words">All words →</a>
      </div>
      {#if recent.length}
        <ul>
          {#each recent as card (card.id)}
            <li>
              <a href="#/words/{card.id}">
                <span class="word"
                  >{card.word}{#if card.part_of_speech}<span class="pos">{posLabel(card.part_of_speech)}</span>{/if}</span
                >
                <span class="meaning">{card.meaning}</span>
              </a>
            </li>
          {/each}
        </ul>
      {:else}
        <p class="muted">No words yet. <a href="#/words">Add your first one.</a></p>
      {/if}
    </section>
  </div>
{:else if !error}
  <p class="muted">Loading…</p>
{/if}

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
  .two {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 20px;
    margin-bottom: 20px;
  }
  .two > .panel {
    margin-bottom: 0;
  }
  @media (max-width: 1000px) {
    .two {
      grid-template-columns: minmax(0, 1fr);
    }
  }

  /* Due today */
  .due {
    display: flex;
    gap: 20px;
    min-height: 280px;
    overflow: hidden;
  }
  .due-main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .due-count {
    display: flex;
    align-items: flex-end;
    gap: 14px;
    margin-top: auto;
  }
  .big {
    font-family: var(--display);
    font-weight: 800;
    font-size: 128px;
    line-height: 0.8;
    letter-spacing: -0.06em;
    color: var(--accent);
  }
  .big.zero {
    color: var(--muted);
  }
  .big-label {
    font-size: 18px;
    font-weight: 500;
    line-height: 1.25;
  }
  .kind {
    height: 24px;
    padding: 0 10px;
    border-radius: 12px;
    background: var(--hover);
    font-size: 12px;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--text-2);
  }
  .dot {
    width: 6px;
    height: 6px;
    border-radius: 3px;
  }
  .dot.new {
    background: var(--text);
  }
  .dot.review {
    background: var(--accent);
  }
  .dot.again {
    background: var(--again);
  }
  .go {
    width: fit-content;
  }
  .deck {
    width: 170px;
    height: 150px;
    flex-shrink: 0;
    align-self: center;
    position: relative;
  }
  .deck-card {
    position: absolute;
    width: 148px;
    height: 100px;
    border-radius: 14px;
  }
  .back2 {
    left: 18px;
    top: 26px;
    border: 1px solid var(--border-strong);
    background: #1a1c22;
    transform: rotate(9deg);
  }
  .back1 {
    left: 10px;
    top: 24px;
    border: 1px solid #353a45;
    background: var(--surface-2);
    transform: rotate(3deg);
  }
  .front {
    left: 2px;
    top: 22px;
    padding: 14px;
    border: 1px solid var(--accent);
    background: #20232b;
    transform: rotate(-4deg);
    display: flex;
    flex-direction: column;
    justify-content: space-between;
  }
  .front .eyebrow {
    font-size: 9px;
  }
  .deck-word {
    font-family: var(--display);
    font-weight: 700;
    font-size: 22px;
    letter-spacing: -0.02em;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  @media (max-width: 560px) {
    .deck {
      display: none;
    }
    .big {
      font-size: 96px;
    }
  }

  /* Latest writing */
  .latest {
    display: flex;
    flex-direction: column;
    gap: 14px;
    min-height: 280px;
  }
  .more {
    font-size: 14px;
    font-weight: 600;
    text-decoration: none;
  }
  .prompt {
    margin: 0;
    font-family: var(--display);
    font-weight: 500;
    font-size: 19px;
    line-height: 1.32;
    display: -webkit-box;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .progress-block {
    margin-top: auto;
    display: grid;
    gap: 8px;
  }
  .mono {
    font-family: var(--mono);
  }
  .bar {
    height: 8px;
    border-radius: 4px;
    background: var(--border);
    overflow: hidden;
  }
  .bar div {
    height: 100%;
    border-radius: 4px;
    background: var(--accent);
  }

  /* Calendar */
  .calendar-head {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 16px;
    flex-wrap: wrap;
    margin-bottom: 16px;
  }
  .calendar-head h2 {
    margin-bottom: 4px;
  }
  .calendar-head p {
    margin: 0;
  }
  .stats {
    display: flex;
    gap: 28px;
  }
  .stats div {
    display: grid;
    justify-items: end;
  }
  .stat {
    font-family: var(--display);
    font-weight: 800;
    font-size: 30px;
    line-height: 1;
    letter-spacing: -0.03em;
  }
  .stat.lime {
    color: var(--accent);
  }
  .eyebrow-ish {
    font-size: 12px;
    color: var(--faint);
  }

  /* Recently added */
  .recent ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .recent li + li {
    border-top: 1px solid var(--line);
  }
  .recent a {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 16px;
    padding: 10px 0;
    color: var(--text);
    text-decoration: none;
  }
  .recent a:hover .word {
    color: var(--accent);
  }
  .word {
    font-family: var(--display);
    font-weight: 700;
    font-size: 17px;
  }
  .pos {
    margin-left: 6px;
    font-family: var(--sans);
    font-weight: 500;
    font-size: 13px;
    font-style: italic;
    color: var(--muted);
  }
  .meaning {
    font-size: 13px;
    color: var(--muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
