<script lang="ts">
  import { onMount } from 'svelte';
  import type { DayActivity } from '../types';

  // GitHub-style practice calendar: 53 week columns x 7 days (Mon-Sun).
  // No streaks: each day stands on its own. The grid stretches to the full
  // width; on narrow screens it keeps a minimum width and scrolls sideways.
  interface Props {
    /** First date shown (a Monday), "YYYY-MM-DD". */
    from: string;
    today: string;
    /** Days with activity; every other date is an empty cell. */
    days: DayActivity[];
  }

  let { from, today, days }: Props = $props();

  const WEEKS = 53;
  const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

  /** Date arithmetic in UTC, so the browser's time zone can't shift a day. */
  function addDays(iso: string, n: number): string {
    const [y, m, d] = iso.split('-').map(Number);
    return new Date(Date.UTC(y, m - 1, d + n)).toISOString().slice(0, 10);
  }

  function label(iso: string, day: DayActivity | undefined): string {
    const [y, m, d] = iso.split('-').map(Number);
    const date = new Date(Date.UTC(y, m - 1, d)).toLocaleDateString('en-GB', {
      weekday: 'short',
      day: 'numeric',
      month: 'short',
      timeZone: 'UTC',
    });
    if (!day) return `${date}: no practice`;
    const parts = [];
    if (day.reviews) parts.push(`${day.reviews} reviews`);
    if (day.writing) parts.push(`${day.writing} writing`);
    if (day.practice_minutes) parts.push(`${day.practice_minutes} min practice`);
    return `${date}: ${parts.join(' · ')}`;
  }

  const byDate = $derived(new Map(days.map((d) => [d.date, d])));

  // Cells in column order: week 0 Mon..Sun, week 1 Mon..Sun, ...
  const cells = $derived.by(() => {
    const list = [];
    for (let i = 0; i < WEEKS * 7; i++) {
      const date = addDays(from, i);
      const day = byDate.get(date);
      list.push({
        date,
        level: day?.level ?? 0,
        future: date > today,
        isToday: date === today,
        title: label(date, day),
      });
    }
    return list;
  });

  // A month name above the first week that starts in that month, as a
  // percentage of the grid width; skipped when the next one is too close.
  const months = $derived.by(() => {
    const found: { week: number; name: string; left: number }[] = [];
    let last = -1;
    for (let w = 0; w < WEEKS; w++) {
      const month = Number(addDays(from, w * 7).slice(5, 7)) - 1;
      if (month !== last) {
        found.push({ week: w, name: MONTHS[month], left: (w / WEEKS) * 100 });
        last = month;
      }
    }
    return found.filter((m, i) => !found[i + 1] || found[i + 1].week - m.week >= 3);
  });

  let scroller: HTMLDivElement | undefined = $state();

  // When the grid scrolls sideways, start at the recent end.
  onMount(() => {
    if (scroller) scroller.scrollLeft = scroller.scrollWidth;
  });
</script>

<div class="scroller" bind:this={scroller}>
  <div class="heatmap">
    <div class="months" aria-hidden="true">
      {#each months as m (m.week)}
        <span style:left="{m.left}%">{m.name}</span>
      {/each}
    </div>
    <div class="body">
      <div class="weekdays" aria-hidden="true">
        <span>Mon</span><span></span><span>Wed</span><span></span><span>Fri</span><span></span><span>Sun</span>
      </div>
      <div class="grid" role="img" aria-label="Practice calendar for the last 53 weeks">
        {#each cells as cell (cell.date)}
          <div
            class="cell l{cell.level}"
            class:future={cell.future}
            class:today={cell.isToday}
            title={cell.future ? '' : cell.title}
          ></div>
        {/each}
      </div>
    </div>
  </div>
</div>
<div class="legend" aria-hidden="true">
  <span>Less</span>
  <span class="cell l0"></span><span class="cell l1"></span><span class="cell l2"></span>
  <span class="cell l3"></span><span class="cell l4"></span>
  <span>More</span>
</div>

<style>
  .scroller {
    overflow-x: auto;
    padding: 2px 0 4px;
  }
  .heatmap {
    min-width: 700px;
    display: grid;
    gap: 6px;
  }
  .months {
    position: relative;
    height: 14px;
    margin-left: 36px;
    font-family: var(--mono);
    font-size: 11px;
    line-height: 14px;
    color: var(--faint);
  }
  .months span {
    position: absolute;
    top: 0;
  }
  .body {
    display: flex;
    align-items: stretch;
  }
  .weekdays {
    width: 36px;
    flex-shrink: 0;
    display: grid;
    grid-template-rows: repeat(7, minmax(0, 1fr));
    row-gap: 3px;
    font-family: var(--mono);
    font-size: 10px;
    color: var(--faint);
  }
  .weekdays span {
    align-self: center;
  }
  .grid {
    flex: 1;
    min-width: 0;
    display: grid;
    grid-template-columns: repeat(53, minmax(0, 1fr));
    grid-template-rows: repeat(7, auto);
    grid-auto-flow: column;
    gap: 3px;
  }
  .cell {
    aspect-ratio: 1;
    border-radius: 4px;
    border: 1px solid transparent;
  }
  .l0 {
    background: #1c1f26;
  }
  .l1 {
    background: #2e4420;
  }
  .l2 {
    background: #4c7428;
  }
  .l3 {
    background: #86b83c;
  }
  .l4 {
    background: var(--accent);
  }
  .future {
    background: transparent;
    border-color: var(--border);
  }
  .today {
    border-color: var(--text);
  }
  .legend {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: 4px;
    margin-top: 6px;
    font-size: 11px;
    color: var(--faint);
  }
  .legend .cell {
    width: 13px;
    height: 13px;
    border-radius: 3px;
    border: none;
  }
  .legend span:first-child {
    margin-right: 4px;
  }
  .legend span:last-child {
    margin-left: 4px;
  }
</style>
