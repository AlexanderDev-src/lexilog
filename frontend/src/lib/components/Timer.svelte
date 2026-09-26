<script lang="ts">
  import { formatClock } from '../format';
  import Icon from './Icon.svelte';

  interface Props {
    /** Countdown length; null = stopwatch that counts up. */
    limitMinutes: number | null;
    /** Seconds used so far. Bound, so the editor can save it. */
    elapsed: number;
    onpause?: () => void;
  }

  let { limitMinutes, elapsed = $bindable(0), onpause }: Props = $props();

  let running = $state(false);
  let interval: ReturnType<typeof setInterval> | undefined;

  const limit = $derived(limitMinutes === null ? null : limitMinutes * 60);
  const remaining = $derived(limit === null ? null : limit - elapsed);
  const overtime = $derived(remaining !== null && remaining < 0);

  function start() {
    if (running) return;
    running = true;
    // Work from the real clock, so a busy tab doesn't make the timer drift.
    const startedAt = Date.now() - elapsed * 1000;
    interval = setInterval(() => {
      elapsed = Math.floor((Date.now() - startedAt) / 1000);
    }, 250);
  }

  function pause() {
    if (!running) return;
    running = false;
    clearInterval(interval);
    onpause?.();
  }

  function reset() {
    pause();
    elapsed = 0;
  }

  // Stop the interval when the component is removed.
  $effect(() => () => clearInterval(interval));
</script>

<div class="timer" class:overtime class:running>
  <span class="icon"><Icon name="clock" size={18} stroke={2} /></span>
  <span class="clock" title={limit === null ? 'Time spent' : 'Time left'}>
    {#if remaining === null}
      {formatClock(elapsed)}
    {:else}
      {overtime ? '+' : ''}{formatClock(remaining)}
    {/if}
  </span>
  {#if running}
    <button type="button" onclick={pause}>Pause</button>
  {:else}
    <button type="button" onclick={start}>{elapsed ? 'Resume' : 'Start'}</button>
  {/if}
  {#if elapsed && !running}
    <button type="button" class="ghost" onclick={reset}>Reset</button>
  {/if}
</div>

<style>
  .timer {
    height: 44px;
    padding: 0 5px 0 12px;
    border-radius: 12px;
    background: var(--bg);
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .icon {
    display: flex;
    color: var(--faint);
  }
  .running .icon {
    color: var(--accent);
  }
  .clock {
    min-width: 72px;
    font-family: var(--mono);
    font-size: 22px;
    font-weight: 600;
    letter-spacing: -0.02em;
    font-variant-numeric: tabular-nums;
  }
  .overtime .clock,
  .overtime .icon {
    color: var(--again);
  }
  button {
    min-height: 34px;
    padding: 0 12px;
    border-radius: 9px;
    font-size: 13px;
  }
</style>
