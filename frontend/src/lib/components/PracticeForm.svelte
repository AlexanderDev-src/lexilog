<script lang="ts">
  import { untrack } from 'svelte';
  import { practiceApi } from '../api/practice';
  import { SKILLS } from '../format';
  import type { PracticeSession, Skill } from '../types';

  interface Props {
    /** "YYYY-MM-DD": default date and the latest date allowed. */
    today: string;
    /** Set to edit an existing session instead of logging a new one. */
    session?: PracticeSession | null;
    /** Hide the date field (the quick form on the dashboard always logs today). */
    showDate?: boolean;
    onsaved?: (session: PracticeSession) => void;
    oncancel?: () => void;
  }

  let { today, session = null, showDate = true, onsaved, oncancel }: Props = $props();

  // Starting values are copied once; `{#key}` in the parent remounts the
  // form when a different session is picked for editing.
  const start = untrack(() => session);
  let skill = $state<Skill>(start?.skill ?? 'listening');
  let minutes = $state<number>(start?.minutes ?? 30);
  let date = $state(start?.practiced_on ?? untrack(() => today));
  let note = $state(start?.note ?? '');

  let saving = $state(false);
  let error = $state('');
  let saved = $state(false);

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    saving = true;
    error = '';
    try {
      const input = { skill, minutes: Number(minutes), practiced_on: date, note };
      const result = start ? await practiceApi.update(start.id, input) : await practiceApi.create(input);
      if (!start) {
        note = '';
        saved = true;
        setTimeout(() => (saved = false), 2500);
      }
      onsaved?.(result);
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      saving = false;
    }
  }
</script>

<form class="practice" onsubmit={submit}>
  <div class="skills" role="radiogroup" aria-label="Skill">
    {#each SKILLS as s (s.value)}
      <button
        type="button"
        role="radio"
        aria-checked={skill === s.value}
        class="skill"
        class:active={skill === s.value}
        onclick={() => (skill = s.value)}>{s.label}</button
      >
    {/each}
  </div>
  <div class="fields" class:no-date={!showDate}>
    <label class="minutes">
      Minutes
      <input type="number" min="1" max="600" required bind:value={minutes} />
    </label>
    {#if showDate}
      <label class="date">
        Date
        <input type="date" max={today} required bind:value={date} />
      </label>
    {/if}
    <label class="note">
      Note
      <input bind:value={note} placeholder="Cambridge 18, Test 3, Section 4" />
    </label>
    <div class="actions">
      {#if oncancel}<button type="button" class="ghost" onclick={oncancel}>Cancel</button>{/if}
      <button type="submit" class="light" disabled={saving}>
        {saving ? 'Saving…' : start ? 'Save' : 'Log'}
      </button>
    </div>
  </div>
  {#if saved}<p class="success small">Logged. It's on your calendar.</p>{/if}
  {#if error}<p class="error small">{error}</p>{/if}
</form>

<style>
  .practice {
    display: grid;
    gap: 14px;
    /* Lets the fields below adapt to the form's own width (container query). */
    container-type: inline-size;
  }
  .skills {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .skill {
    min-height: 40px;
    padding: 0 14px;
    border-radius: 10px;
    color: var(--text-2);
    font-weight: 500;
  }
  .skill.active {
    border-color: var(--accent);
    background: var(--accent-soft);
    color: var(--accent);
    font-weight: 600;
  }
  .fields {
    display: grid;
    grid-template-columns: 96px auto minmax(0, 1fr) auto;
    align-items: end;
    gap: 10px;
  }
  .fields.no-date {
    grid-template-columns: 96px minmax(0, 1fr) auto;
  }
  .minutes input {
    font-family: var(--mono);
    font-weight: 500;
  }
  .date input {
    width: 100%;
  }
  .actions {
    display: flex;
    gap: 6px;
  }
  .practice p {
    margin: 0;
  }
  /* Two rows when the form is narrow, e.g. in the dashboard's half-width card. */
  @container (max-width: 560px) {
    .fields:not(.no-date) {
      grid-template-columns: 96px minmax(0, 1fr);
    }
    .fields:not(.no-date) .note {
      grid-column: 1 / -1;
      order: 3;
    }
    .fields:not(.no-date) .actions {
      order: 4;
      grid-column: 1 / -1;
      justify-content: flex-end;
    }
  }
</style>
