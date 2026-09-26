<script lang="ts">
  import { mistakesApi } from '../lib/api/mistakes';
  import { formatPeriod } from '../lib/format';
  import type { MistakeTrend, TagTrend, TrendBucket } from '../lib/types';

  let bucket = $state<TrendBucket>('month');
  let trend = $state<MistakeTrend | null>(null);
  let error = $state('');

  // Reload whenever the bucket changes.
  $effect(() => {
    const chosen = bucket;
    mistakesApi
      .trend(chosen)
      .then((t) => (trend = t))
      .catch((err) => (error = err instanceof Error ? err.message : String(err)));
  });

  const DIRECTION = {
    fading: { label: 'Fading', icon: '↓' },
    rising: { label: 'Rising', icon: '↑' },
    steady: { label: 'Steady', icon: '→' },
    new: { label: 'New', icon: '•' },
  } as const;

  /** Bar heights in % of the tag's own highest rate, so each row shows its own shape. */
  function bars(tag: TagTrend) {
    const max = Math.max(...tag.points.map((p) => p.per_1000_words), 0.0001);
    return tag.points.map((p) => ({ ...p, height: Math.round((p.per_1000_words / max) * 100) }));
  }

  const latestRate = (tag: TagTrend) => tag.points.at(-1)?.per_1000_words ?? 0;
</script>

<div class="row spread wrap head">
  <div>
    <span class="eyebrow">Mistake log</span>
    <h1>Mistakes</h1>
  </div>
  <div class="row" role="radiogroup" aria-label="Group by">
    <button type="button" role="radio" aria-checked={bucket === 'month'} class="chip" class:active={bucket === 'month'} onclick={() => (bucket = 'month')}>By month</button>
    <button type="button" role="radio" aria-checked={bucket === 'week'} class="chip" class:active={bucket === 'week'} onclick={() => (bucket = 'week')}>By week</button>
  </div>
</div>
<p class="muted">
  How often each tagged mistake appears per 1000 words of first drafts. Only pieces you checked (tagged,
  or with feedback) count. Lower is better.
</p>

{#if error}<p class="error">{error}</p>{/if}

{#if trend === null}
  <p class="muted">Loading…</p>
{:else if trend.tags.length === 0}
  <section class="panel">
    <p>No mistakes tagged yet.</p>
    <p class="muted small">Open a piece in <a href="#/writing">Writing</a> and tag its mistakes, or use AI feedback to suggest them.</p>
  </section>
{:else}
  <section class="panel">
    <div class="axis muted small">
      <span>{formatPeriod(trend.periods[0].period)}</span>
      <span>{formatPeriod(trend.periods.at(-1)!.period)}</span>
    </div>
    <ul class="tags">
      {#each trend.tags as tag (tag.tag)}
        <li>
          <div class="name">
            <strong>{tag.tag}</strong>
            <span class="muted small">{tag.total} in total</span>
          </div>
          <div class="bars" aria-label="{tag.tag}: {latestRate(tag)} per 1000 words in the latest period">
            {#each bars(tag) as b (b.period)}
              <span
                class="bar"
                style:height="{Math.max(b.height, b.count ? 6 : 2)}%"
                class:empty={b.count === 0}
                title="{formatPeriod(b.period)}: {b.count} ({b.per_1000_words} per 1000 words)"
              ></span>
            {/each}
          </div>
          <div class="latest">
            <span class="rate">{latestRate(tag)}</span>
            <span class="muted small">per 1000 now</span>
          </div>
          <span class="direction {tag.direction}">{DIRECTION[tag.direction].icon} {DIRECTION[tag.direction].label}</span>
        </li>
      {/each}
    </ul>
  </section>
{/if}

<style>
  .head {
    align-items: flex-end;
    margin-bottom: 12px;
  }
  .head h1 {
    margin: 8px 0 0;
  }
  .name strong {
    font-family: var(--display);
    font-size: 17px;
  }
  .rate {
    font-family: var(--display);
    font-size: 20px;
  }
  .axis {
    display: flex;
    justify-content: space-between;
    margin: 0 13rem 8px 12rem;
  }
  .tags {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .tags li {
    display: grid;
    grid-template-columns: 11rem 1fr 6rem 5rem;
    align-items: center;
    gap: 16px;
    padding: 12px 0;
  }
  .tags li + li {
    border-top: 1px solid var(--border);
  }
  .name {
    display: grid;
  }
  /* space-between puts the first bar at the left edge and the last at the
     right edge, matching the first and last period labels above. */
  .bars {
    height: 40px;
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 3px;
  }
  .bar {
    flex: 0 1 24px;
    border-radius: 3px 3px 0 0;
    background: var(--accent);
  }
  .bar.empty {
    background: var(--border);
  }
  .latest {
    display: grid;
    text-align: right;
  }
  .rate {
    font-weight: 700;
    font-variant-numeric: tabular-nums;
  }
  .direction {
    font-size: 0.85rem;
    font-weight: 600;
    text-align: right;
  }
  .fading {
    color: var(--success);
  }
  .rising {
    color: var(--again);
  }
  .steady,
  .new {
    color: var(--muted);
  }
  @media (max-width: 640px) {
    .axis {
      display: none;
    }
    .tags li {
      grid-template-columns: 1fr auto;
    }
    .bars {
      grid-column: 1 / -1;
      order: 3;
    }
  }
</style>
