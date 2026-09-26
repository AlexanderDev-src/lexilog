<script lang="ts">
  // Tags as removable chips. Enter or comma adds the typed tag; Backspace on
  // an empty field removes the last one. Enter on an empty field submits the form.
  interface Props {
    tags: string[];
    suggestions?: string[];
    id?: string;
  }

  let { tags = $bindable([]), suggestions = [], id = 'tags' }: Props = $props();
  let draft = $state('');

  const listId = $derived(`${id}-suggestions`);
  const unused = $derived(suggestions.filter((s) => !tags.includes(s)));

  function add(raw: string) {
    const tag = raw.trim().toLowerCase();
    if (tag && !tags.includes(tag)) tags = [...tags, tag];
    draft = '';
  }

  function remove(tag: string) {
    tags = tags.filter((t) => t !== tag);
  }

  function onkeydown(event: KeyboardEvent) {
    if ((event.key === 'Enter' || event.key === ',') && draft.trim()) {
      event.preventDefault();
      add(draft);
    } else if (event.key === ',') {
      event.preventDefault();
    } else if (event.key === 'Backspace' && !draft && tags.length) {
      tags = tags.slice(0, -1);
    }
  }
</script>

<div class="tag-input">
  {#each tags as tag (tag)}
    <span class="chip">
      {tag}
      <button type="button" class="chip-x" aria-label="Remove {tag}" onclick={() => remove(tag)}>×</button>
    </span>
  {/each}
  <input
    {id}
    list={listId}
    bind:value={draft}
    {onkeydown}
    onblur={() => draft && add(draft)}
    placeholder={tags.length ? '' : 'environment, education…'}
    autocomplete="off"
  />
  <datalist id={listId}>
    {#each unused as suggestion (suggestion)}
      <option value={suggestion}></option>
    {/each}
  </datalist>
</div>

<style>
  .tag-input {
    min-height: 50px;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    align-items: center;
    padding: 8px 10px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    background: var(--input-bg);
  }
  .tag-input:focus-within {
    border-color: var(--accent);
    box-shadow: 0 0 0 4px var(--accent-soft);
  }
  .chip {
    min-height: 30px;
    padding: 0 4px 0 12px;
    border: none;
    background: var(--hover);
    color: var(--text);
    font-size: 13px;
  }
  input {
    flex: 1;
    min-width: 8rem;
    border: none;
    padding: 4px 2px;
    background: transparent;
    box-shadow: none;
  }
  input:focus {
    box-shadow: none;
  }
  .chip-x {
    min-height: 0;
    width: 24px;
    height: 24px;
    padding: 0;
    border: none;
    border-radius: 12px;
    color: var(--muted);
    font-size: 1rem;
    line-height: 1;
  }
  .chip-x:hover:not(:disabled) {
    color: var(--text);
  }
</style>
