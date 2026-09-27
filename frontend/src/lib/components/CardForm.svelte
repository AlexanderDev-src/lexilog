<script lang="ts">
  import { untrack } from 'svelte';
  import { PARTS_OF_SPEECH } from '../format';
  import type { CardInput, PartOfSpeech } from '../types';
  import TagInput from './TagInput.svelte';

  interface Props {
    initial?: Partial<CardInput>;
    submitLabel?: string;
    tagSuggestions?: string[];
    /**
     * Add mode: after saving, clear word/part of speech/meaning/example but
     * keep source and tags, since the next word often comes from the same article.
     */
    keepContext?: boolean;
    autofocus?: boolean;
    onsave: (input: CardInput) => Promise<void>;
  }

  let {
    initial = {},
    submitLabel = 'Save',
    tagSuggestions = [],
    keepContext = false,
    autofocus = false,
    onsave,
  }: Props = $props();

  // Copy the starting values once; the form edits its own state after that.
  // `untrack` says "read this now, don't follow later changes".
  const start = untrack(() => ({
    word: '',
    part_of_speech: '' as PartOfSpeech | '',
    meaning: '',
    example: '',
    source: '',
    tags: [],
    ...initial,
  }));
  let word = $state(start.word);
  let partOfSpeech = $state(start.part_of_speech);
  let meaning = $state(start.meaning);
  let example = $state(start.example);
  let source = $state(start.source);
  let tags = $state<string[]>([...start.tags]);

  let saving = $state(false);
  let error = $state('');
  let wordInput: HTMLInputElement | undefined = $state();

  $effect(() => {
    if (autofocus) wordInput?.focus();
  });

  async function submit(event?: SubmitEvent) {
    event?.preventDefault();
    if (!word.trim()) {
      error = 'Word is required';
      wordInput?.focus();
      return;
    }
    saving = true;
    error = '';
    try {
      await onsave({ word, part_of_speech: partOfSpeech, meaning, example, source, tags });
      if (keepContext) {
        word = meaning = example = partOfSpeech = '';
        wordInput?.focus();
      }
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      saving = false;
    }
  }

  // In the textareas, Ctrl+Enter (or Cmd+Enter) saves.
  function saveShortcut(event: KeyboardEvent) {
    if (event.key === 'Enter' && (event.ctrlKey || event.metaKey)) {
      event.preventDefault();
      submit();
    }
  }
</script>

<form class="card-form" onsubmit={submit}>
  <div class="word-row">
    <label>
      Word
      <input
        class="word"
        bind:this={wordInput}
        bind:value={word}
        autocomplete="off"
        spellcheck="false"
        placeholder="e.g. alleviate"
      />
    </label>
    <label>
      Part of speech
      <select class="pos" bind:value={partOfSpeech}>
        <option value="">—</option>
        {#each PARTS_OF_SPEECH as p (p.value)}
          <option value={p.value} title={p.name}>{p.short}</option>
        {/each}
      </select>
    </label>
  </div>
  <label>
    Meaning
    <textarea bind:value={meaning} rows="2" onkeydown={saveShortcut}></textarea>
  </label>
  <label>
    Example sentence
    <textarea
      class="example"
      bind:value={example}
      rows="3"
      onkeydown={saveShortcut}
      placeholder="The real sentence where you found it"
    ></textarea>
  </label>
  <label>
    Source
    <input bind:value={source} placeholder="Guardian, Cambridge 18 Test 2…" />
  </label>
  <div class="topics">
    <label for="card-tags">IELTS topics</label>
    <TagInput id="card-tags" bind:tags suggestions={tagSuggestions} />
  </div>

  <div class="submit">
    <button type="submit" class="primary" disabled={saving}>{saving ? 'Saving…' : submitLabel}</button>
    <kbd class="key" title="Ctrl+Enter saves from any text box">Ctrl ↵</kbd>
  </div>
  {#if error}<p class="error small">{error}</p>{/if}
</form>

<style>
  .card-form {
    display: grid;
    gap: 16px;
  }
  .word-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 12px;
  }
  .word {
    height: 64px;
    padding: 0 18px;
    border-radius: 14px;
    font-family: var(--display);
    font-weight: 700;
    font-size: 28px;
    letter-spacing: -0.02em;
  }
  .pos {
    height: 64px;
    padding: 0 14px;
    border-radius: 14px;
    font-weight: 600;
  }
  textarea {
    line-height: 1.5;
    resize: none;
  }
  .example {
    font-style: italic;
  }
  .topics {
    display: grid;
    gap: 6px;
  }
  .submit {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-top: 4px;
  }
  .submit .primary {
    flex: 1;
    min-height: 52px;
  }
  .submit kbd {
    height: 52px;
    padding: 0 14px;
    border-radius: 14px;
  }
  .card-form p {
    margin: 0;
  }
</style>
