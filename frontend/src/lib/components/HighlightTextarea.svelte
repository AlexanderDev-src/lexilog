<script lang="ts" module>
  export interface Mark {
    start: number;
    end: number;
    /** "deck" = a deck word (dotted underline); "focus" = the quote an AI issue points at. */
    kind: 'deck' | 'focus';
  }
</script>

<script lang="ts">
  import { tick } from 'svelte';

  // A textarea that can underline parts of its text. A textarea can't style
  // its own text, so a "mirror" div with the same text and the same font sits
  // underneath: its text is invisible, only the <mark> underlines show
  // through the transparent textarea on top.
  //
  // The mirror is in the normal layout, so it also sets the height: the
  // textarea grows with its text and never scrolls inside itself (the page
  // scrolls instead, which keeps the side panel next to it in view).

  interface Props {
    value: string;
    marks: Mark[];
    textarea?: HTMLTextAreaElement;
    id?: string;
    placeholder?: string;
    oninput?: () => void;
    onblur?: () => void;
    onselect?: () => void;
  }

  let {
    value = $bindable(),
    marks,
    textarea = $bindable(),
    id,
    placeholder,
    oninput,
    onblur,
    onselect,
  }: Props = $props();

  let mirror: HTMLDivElement | undefined = $state();

  // Splits the text at every mark boundary. Each piece knows which kinds of
  // mark cover it, so overlapping marks (a deck word inside a quote) both show.
  const pieces = $derived.by(() => {
    const cuts = new Set([0, value.length]);
    for (const m of marks) {
      cuts.add(Math.max(0, Math.min(value.length, m.start)));
      cuts.add(Math.max(0, Math.min(value.length, m.end)));
    }
    const points = [...cuts].sort((a, b) => a - b);
    const result: { text: string; deck: boolean; focus: boolean }[] = [];
    for (let i = 0; i < points.length - 1; i++) {
      const [a, b] = [points[i], points[i + 1]];
      const covers = (kind: Mark['kind']) => marks.some((m) => m.kind === kind && m.start <= a && m.end >= b);
      result.push({ text: value.slice(a, b), deck: covers('deck'), focus: covers('focus') });
    }
    return result;
  });

  /** Scrolls the page so the "focus" mark is in the middle of the screen. */
  export async function revealFocus() {
    await tick();
    mirror?.querySelector('mark.focus')?.scrollIntoView({ block: 'center', behavior: 'smooth' });
  }
</script>

<div class="hl">
  <!-- Must stay on one line: any spaces between the tags would show as text. -->
  <div class="mirror" bind:this={mirror} aria-hidden="true">{#each pieces as p, i (i)}{#if p.deck || p.focus}<mark class:deck={p.deck} class:focus={p.focus}>{p.text}</mark>{:else}{p.text}{/if}{/each}{'\n​'}</div>
  <textarea
    {id}
    bind:this={textarea}
    bind:value
    {placeholder}
    {oninput}
    {onblur}
    {onselect}
    onscroll={(e) => (e.currentTarget.scrollTop = 0)}
    spellcheck="true"
  ></textarea>
</div>

<style>
  .hl {
    position: relative;
    font-size: 18px;
    line-height: 1.8;
  }
  /* Both layers must lay out text identically, or the underlines drift. */
  .mirror,
  textarea {
    margin: 0;
    padding: 0;
    border: none;
    font: inherit;
    letter-spacing: normal;
    white-space: pre-wrap;
    overflow-wrap: break-word;
    word-break: normal;
    tab-size: 4;
  }
  .mirror {
    min-height: 60vh;
    color: transparent;
  }
  textarea {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    overflow: hidden;
    resize: none;
    background: transparent;
    color: #e4e1d9;
  }
  textarea:focus {
    box-shadow: none;
  }
  mark {
    color: transparent;
    background: none;
  }
  mark.deck {
    text-decoration: underline dotted var(--accent);
    text-decoration-thickness: 2px;
    text-underline-offset: 5px;
  }
  mark.focus {
    border-radius: 3px;
    background: var(--again-soft);
    box-shadow: 0 0 0 2px var(--again-soft);
  }
</style>
