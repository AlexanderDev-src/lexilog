<script lang="ts">
  import { onMount } from 'svelte';
  import { practiceApi } from '../lib/api/practice';
  import PracticeForm from '../lib/components/PracticeForm.svelte';
  import { formatDate, SKILLS } from '../lib/format';
  import type { PracticeSession } from '../lib/types';

  let sessions = $state<PracticeSession[] | null>(null);
  let editing = $state<PracticeSession | null>(null);
  let error = $state('');

  // Local date, "YYYY-MM-DD". The server checks it again with APP_TZ.
  const today = new Date().toLocaleDateString('sv-SE');

  async function load() {
    try {
      sessions = await practiceApi.list(200);
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    }
  }

  onMount(load);

  async function remove(session: PracticeSession) {
    if (!confirm(`Delete ${session.minutes} min of ${session.skill} on ${formatDate(session.practiced_on)}?`)) return;
    try {
      await practiceApi.remove(session.id);
      if (editing?.id === session.id) editing = null;
      await load();
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    }
  }

  function saved() {
    editing = null;
    load();
  }

  const skillLabel = (value: string) => SKILLS.find((s) => s.value === value)?.label ?? value;

  // Minutes per skill over the last 7 days.
  const lastWeek = $derived.by(() => {
    const since = new Date(Date.now() - 6 * 86_400_000).toLocaleDateString('sv-SE');
    const totals = new Map<string, number>();
    for (const s of sessions ?? []) {
      if (s.practiced_on >= since) totals.set(s.skill, (totals.get(s.skill) ?? 0) + s.minutes);
    }
    return SKILLS.filter((s) => totals.has(s.value)).map((s) => ({ label: s.label, minutes: totals.get(s.value)! }));
  });
</script>

<header class="page-head">
  <span class="eyebrow">Listening · reading · speaking · other</span>
  <h1>Practice log</h1>
  <p class="muted">Practice outside this app. Every session shows on the calendar.</p>
</header>

<section class="panel">
  <h2>{editing ? 'Edit practice' : 'Log practice'}</h2>
  {#key editing?.id}
    <PracticeForm {today} session={editing} onsaved={saved} oncancel={editing ? () => (editing = null) : undefined} />
  {/key}
</section>

{#if error}<p class="error">{error}</p>{/if}

<section class="panel">
  <div class="row spread wrap">
    <h2>History</h2>
    {#if lastWeek.length}
      <span class="muted small">
        Last 7 days: {lastWeek.map((w) => `${w.label} ${w.minutes} min`).join(' · ')}
      </span>
    {/if}
  </div>
  {#if sessions === null}
    <p class="muted">Loading…</p>
  {:else if sessions.length === 0}
    <p class="muted">Nothing logged yet.</p>
  {:else}
    <ul class="sessions">
      {#each sessions as s (s.id)}
        <li class:editing={editing?.id === s.id}>
          <span class="when muted small">{formatDate(s.practiced_on)}</span>
          <span class="badge">{skillLabel(s.skill)}</span>
          <span class="minutes">{s.minutes} min</span>
          <span class="note">{s.note}</span>
          <span class="actions">
            <button type="button" class="ghost small" onclick={() => (editing = s)}>Edit</button>
            <button type="button" class="ghost small danger-text" onclick={() => remove(s)}>Delete</button>
          </span>
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
    margin: 8px 0 8px;
  }
  .page-head p {
    margin: 0;
  }
  .sessions {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .sessions li {
    display: grid;
    grid-template-columns: 7.5rem auto 4.5rem 1fr auto;
    align-items: center;
    gap: 12px;
    padding: 8px 4px;
  }
  .sessions li + li {
    border-top: 1px solid var(--line);
  }
  .sessions li.editing {
    background: var(--accent-soft);
  }
  .minutes {
    font-family: var(--mono);
    font-weight: 600;
  }
  .when {
    font-family: var(--mono);
  }
  .note {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .actions {
    display: flex;
  }
  .actions button {
    padding: 6px 10px;
  }
  @media (max-width: 640px) {
    .sessions li {
      grid-template-columns: auto auto 1fr;
    }
    .note {
      grid-column: 1 / -1;
      white-space: normal;
    }
    .actions {
      grid-column: 1 / -1;
      justify-content: flex-end;
    }
  }
</style>
