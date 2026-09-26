<script lang="ts">
  import { onMount } from 'svelte';
  import { writingApi } from '../lib/api/writing';
  import AiFeedbackPanel from '../lib/components/AiFeedbackPanel.svelte';
  import Icon from '../lib/components/Icon.svelte';
  import MistakeEditor from '../lib/components/MistakeEditor.svelte';
  import Timer from '../lib/components/Timer.svelte';
  import VersionDiff from '../lib/components/VersionDiff.svelte';
  import { countWords, formatTime, PRESETS } from '../lib/format';
  import { navigate } from '../lib/router.svelte';
  import type { MistakeCount, Piece, Version, WritingKind } from '../lib/types';

  let { id }: { id: string } = $props();

  let piece = $state<Piece | null>(null);
  let error = $state('');

  // The version open in the editor.
  let activeId = $state<number | null>(null);
  let body = $state('');
  let feedback = $state(''); // pasted feedback for this version
  let elapsed = $state(0); // timer seconds for this version
  let essay: HTMLTextAreaElement | undefined = $state();
  // The mistake editor component, so the AI panel can add tags to it.
  let mistakeEditor: ReturnType<typeof MistakeEditor> | undefined = $state();

  type SaveState = 'saved' | 'dirty' | 'saving' | 'error';
  let saveState = $state<SaveState>('saved');
  let savedAt = $state<string | null>(null);
  let saveTimer: ReturnType<typeof setTimeout> | undefined;

  // Compare mode: two version ids shown side by side.
  let comparing = $state(false);
  let leftId = $state<number | null>(null);
  let rightId = $state<number | null>(null);

  const active = $derived(piece?.versions.find((v) => v.id === activeId) ?? null);
  const preset = $derived(PRESETS[piece?.kind ?? 'paragraph']);
  const words = $derived(countWords(body));
  const latest = $derived(piece?.versions.at(-1) ?? null);
  const left = $derived(piece?.versions.find((v) => v.id === leftId));
  const right = $derived(piece?.versions.find((v) => v.id === rightId));

  onMount(() => {
    load();
    // Warn before closing the tab with unsaved text.
    const beforeUnload = (event: BeforeUnloadEvent) => {
      if (saveState !== 'saved') event.preventDefault();
    };
    window.addEventListener('beforeunload', beforeUnload);
    return () => {
      window.removeEventListener('beforeunload', beforeUnload);
      // Leaving the page inside the app: save whatever is pending.
      if (saveState === 'dirty') saveBody();
    };
  });

  async function load() {
    try {
      piece = await writingApi.get(Number(id));
      const last = piece.versions.at(-1);
      if (last) open(last);
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    }
  }

  function open(version: Version) {
    activeId = version.id;
    body = version.body;
    feedback = version.feedback;
    elapsed = version.seconds_spent ?? 0;
    saveState = 'saved';
    savedAt = version.updated_at;
    comparing = false;
  }

  async function switchTo(version: Version) {
    if (version.id === activeId && !comparing) return;
    await saveBody();
    open(version);
  }

  // ---- saving ----------------------------------------------------------

  // Typing in the essay or the feedback box: save 1 s after the last keystroke.
  function onBodyInput() {
    saveState = 'dirty';
    clearTimeout(saveTimer);
    saveTimer = setTimeout(saveBody, 1000); // 1 s after the last keystroke
  }

  async function saveBody() {
    clearTimeout(saveTimer);
    if (!piece || !active || saveState === 'saved') return;
    const versionId = active.id;
    const sentBody = body;
    const sentFeedback = feedback;
    saveState = 'saving';
    try {
      const saved = await writingApi.updateVersion(versionId, {
        body: sentBody,
        feedback: sentFeedback,
        seconds_spent: elapsed > 0 ? elapsed : undefined,
      });
      piece.versions = piece.versions.map((v) => (v.id === saved.id ? saved : v));
      savedAt = saved.updated_at;
      // If the user kept typing during the request, there is still more to save.
      const unchanged = body === sentBody && feedback === sentFeedback && activeId === versionId;
      saveState = unchanged ? 'saved' : 'dirty';
    } catch (err) {
      saveState = 'error';
      error = err instanceof Error ? err.message : String(err);
    }
  }

  /** Finds the AI's quote in the essay and selects it (exact match first, then any case). */
  function selectQuote(quote: string) {
    if (!essay) return;
    let start = body.indexOf(quote);
    if (start < 0) start = body.toLowerCase().indexOf(quote.toLowerCase());
    if (start < 0) {
      error = `Couldn't find “${quote}” in this version. It may have been edited since.`;
      return;
    }
    error = '';
    essay.focus();
    essay.setSelectionRange(start, start + quote.length);
    // Scroll the textarea so the selection is visible (roughly by line).
    const line = body.slice(0, start).split('\n').length;
    const lineHeight = parseFloat(getComputedStyle(essay).lineHeight) || 28;
    essay.scrollTop = Math.max(0, (line - 3) * lineHeight);
  }

  function applyTags(counts: MistakeCount[]) {
    mistakeEditor?.merge(counts);
  }

  // Timer paused: store the time even if the text didn't change.
  function onTimerPause() {
    saveState = 'dirty';
    saveBody();
  }

  async function saveDetails() {
    if (!piece) return;
    try {
      const updated = await writingApi.update(piece.id, {
        kind: piece.kind,
        prompt: piece.prompt,
        written_on: piece.written_on,
      });
      piece.prompt = updated.prompt;
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    }
  }

  // ---- versions --------------------------------------------------------

  async function newVersion() {
    if (!piece) return;
    await saveBody();
    const version = await writingApi.addVersion(piece.id);
    piece.versions = [...piece.versions, version];
    open(version);
  }

  async function deleteVersion() {
    const target = active;
    if (!piece || !target || piece.versions.length < 2) return;
    if (!confirm(`Delete version ${target.version_no}? This cannot be undone.`)) return;
    clearTimeout(saveTimer);
    await writingApi.removeVersion(target.id);
    piece.versions = piece.versions.filter((v) => v.id !== target.id);
    const last = piece.versions.at(-1);
    if (last) open(last);
  }

  async function deletePiece() {
    if (!piece || !confirm('Delete this piece and all its versions?')) return;
    clearTimeout(saveTimer);
    saveState = 'saved';
    await writingApi.remove(piece.id);
    navigate('/writing');
  }

  async function startCompare() {
    if (!piece || piece.versions.length < 2) return;
    await saveBody();
    // Default: the open version against the one before it.
    const index = piece.versions.findIndex((v) => v.id === activeId);
    const rightIndex = index > 0 ? index : 1;
    leftId = piece.versions[rightIndex - 1].id;
    rightId = piece.versions[rightIndex].id;
    comparing = true;
  }

  const KINDS: WritingKind[] = ['task1', 'task2', 'paragraph'];

  /** "v2 · 187 words · 23 min" for the compare headers. */
  function versionMeta(v: Version): string {
    const secs = v.seconds_spent ?? 0;
    const minutes = secs === 0 ? '' : secs < 60 ? ' · <1 min' : ` · ${Math.round(secs / 60)} min`;
    return `${v.word_count} words${minutes}`;
  }
</script>

<div class="top-row">
  <a class="back" href="#/writing"><Icon name="arrow-left" size={16} stroke={2.2} /> Writing</a>
  {#if piece}<button type="button" class="small-btn" onclick={deletePiece}>Delete piece</button>{/if}
</div>

{#if error}<p class="error">{error}</p>{/if}

{#if piece}
  <header class="piece-head">
    <div class="meta-row">
      <label class="visually-hidden" for="kind">Type</label>
      <select id="kind" class="kind" bind:value={piece.kind} onchange={saveDetails}>
        {#each KINDS as kind (kind)}<option value={kind}>{PRESETS[kind].label}</option>{/each}
      </select>
      <label class="visually-hidden" for="written-on">Date</label>
      <input id="written-on" class="date" type="date" bind:value={piece.written_on} onchange={saveDetails} />
      {#if preset.minutes}
        <span class="eyebrow">{preset.minutes} min · {preset.minWords} words</span>
      {/if}
    </div>
    <label class="visually-hidden" for="prompt">Question / prompt</label>
    <textarea
      id="prompt"
      class="prompt"
      bind:value={piece.prompt}
      onblur={saveDetails}
      rows="2"
      placeholder="Paste the task question here"
    ></textarea>
  </header>

  <div class="toolbar">
    <div class="tabs" role="tablist" aria-label="Versions">
      {#each piece.versions as version (version.id)}
        <button
          type="button"
          role="tab"
          aria-selected={!comparing && version.id === activeId}
          class="tab"
          class:active={!comparing && version.id === activeId}
          onclick={() => switchTo(version)}
          title="{version.word_count} words">v{version.version_no}</button
        >
      {/each}
    </div>
    <button type="button" class="ghost tool" onclick={newVersion} title="Start a rewrite from the latest version">
      <Icon name="plus" size={16} stroke={2.2} /> New version
    </button>
    {#if piece.versions.length > 1}
      <button type="button" class="tool" class:on={comparing} onclick={() => (comparing ? active && open(active) : startCompare())}>
        <Icon name="compare" size={16} stroke={2} /> {comparing ? 'Back to editor' : 'Compare'}
      </button>
    {/if}
    {#if active && !comparing}
      <div class="tool-right">
        <div class="count">
          <span class:short={preset.minWords !== null && words < preset.minWords} class:enough={preset.minWords !== null && words >= preset.minWords}>
            {words}<span class="of">{preset.minWords !== null ? ` / ${preset.minWords}` : ''} words</span>
          </span>
          {#if preset.minWords !== null}
            <div class="count-bar" class:enough={words >= preset.minWords}>
              <div style:width="{Math.min(100, (words / preset.minWords) * 100)}%"></div>
            </div>
          {/if}
        </div>
        {#key active.id}
          <Timer limitMinutes={preset.minutes} bind:elapsed onpause={onTimerPause} />
        {/key}
        <span class="save">
          {#if saveState === 'saving'}Saving…
          {:else if saveState === 'dirty'}Unsaved
          {:else if saveState === 'error'}<span class="error">Save failed</span>
            <button type="button" class="ghost small-btn" onclick={saveBody}>Retry</button>
          {:else if savedAt}<i class="dot"></i>Saved {formatTime(savedAt)}{/if}
        </span>
      </div>
    {/if}
  </div>

  {#if comparing && left && right}
    <section class="compare">
      <div class="row wrap compare-head">
        <h2>Compare versions</h2>
        <label class="pick">
          From
          <select bind:value={leftId}>
            {#each piece.versions as v (v.id)}<option value={v.id}>v{v.version_no}</option>{/each}
          </select>
        </label>
        <Icon name="arrow-right" />
        <label class="pick">
          To
          <select bind:value={rightId}>
            {#each piece.versions as v (v.id)}<option value={v.id}>v{v.version_no}</option>{/each}
          </select>
        </label>
      </div>
      <VersionDiff
        before={left.body}
        after={right.body}
        beforeLabel="v{left.version_no}"
        afterLabel="v{right.version_no}"
        beforeMeta={versionMeta(left)}
        afterMeta={versionMeta(right)}
      />
    </section>
  {:else if active}
    <div class="body-grid">
      <div class="main-col">
        <article class="essay-card">
          <label class="visually-hidden" for="essay">Essay, version {active.version_no}</label>
          <textarea
            id="essay"
            class="essay"
            bind:this={essay}
            bind:value={body}
            oninput={onBodyInput}
            onblur={saveBody}
            placeholder="Start writing…"
            spellcheck="true"
          ></textarea>
          {#if piece.versions.length > 1}
            <div class="essay-foot">
              <span class="muted small">
                {#if active.id !== latest?.id}You are editing an older version (v{active.version_no}).{/if}
              </span>
              <button type="button" class="ghost small-btn danger-text" onclick={deleteVersion}>Delete v{active.version_no}</button>
            </div>
          {/if}
        </article>

        <section class="panel">
          <h2><Icon name="sparkle" size={18} /> AI feedback on v{active.version_no}</h2>
          {#key active.id}
            <AiFeedbackPanel
              pieceId={piece.id}
              version={active}
              {words}
              beforeReview={saveBody}
              onselectquote={selectQuote}
              onapplytags={applyTags}
            />
          {/key}
        </section>
      </div>

      <aside class="rail">
        <section class="panel feedback-panel">
          <div class="row spread">
            <h2>Feedback on v{active.version_no}</h2>
            <span class="eyebrow">pasted</span>
          </div>
          <label class="visually-hidden" for="feedback">Feedback on version {active.version_no}</label>
          <textarea
            id="feedback"
            class="feedback"
            bind:value={feedback}
            oninput={onBodyInput}
            onblur={saveBody}
            placeholder="Paste feedback from a teacher, an AI chat, or your own notes"
          ></textarea>
        </section>

        <section class="panel">
          <div class="row spread">
            <h2>Mistakes in this piece</h2>
            <a class="small link" href="#/mistakes">Trend →</a>
          </div>
          <MistakeEditor bind:this={mistakeEditor} pieceId={piece.id} />
        </section>
      </aside>
    </div>
  {/if}
{:else if !error}
  <p class="muted">Loading…</p>
{/if}

<style>
  .top-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 16px;
  }
  .back {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-weight: 600;
    font-size: 14px;
    text-decoration: none;
  }
  .small-btn {
    min-height: 36px;
    padding: 0 12px;
    font-size: 13px;
    color: var(--muted);
  }

  .piece-head {
    display: grid;
    gap: 10px;
    margin-bottom: 20px;
  }
  .meta-row {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }
  .kind {
    height: 30px;
    padding: 0 8px;
    border: 1px solid var(--accent);
    border-radius: 6px;
    background: transparent;
    color: var(--accent);
    font-family: var(--mono);
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .date {
    height: 30px;
    padding: 0 8px;
    border-color: transparent;
    background: transparent;
    color: var(--faint);
    font-family: var(--mono);
    font-size: 13px;
  }
  .date:hover {
    border-color: var(--border-strong);
  }
  .prompt {
    padding: 4px 6px;
    margin-left: -6px;
    border-color: transparent;
    background: transparent;
    font-family: var(--display);
    font-weight: 700;
    font-size: clamp(1.3rem, 2.4vw, 1.75rem);
    line-height: 1.25;
    letter-spacing: -0.02em;
    resize: none;
    field-sizing: content;
    min-height: 2.6em;
  }
  .prompt:hover {
    border-color: var(--border);
  }

  .toolbar {
    min-height: 60px;
    padding: 8px;
    margin-bottom: 20px;
    border-radius: 16px;
    background: var(--surface);
    border: 1px solid var(--border);
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }
  .tabs {
    display: flex;
    gap: 4px;
    padding: 4px;
    border-radius: 12px;
    background: var(--bg);
  }
  .tab {
    min-height: 36px;
    padding: 0 14px;
    border: none;
    border-radius: 9px;
    color: var(--muted);
    font-family: var(--mono);
    font-size: 14px;
  }
  .tab.active,
  .tab.active:hover {
    background: var(--text);
    color: var(--on-accent);
  }
  .tool {
    min-height: 40px;
    padding: 0 12px;
    font-size: 14px;
  }
  .tool.on {
    border-color: var(--accent);
    color: var(--accent);
  }
  .tool-right {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 22px;
    flex-wrap: wrap;
  }
  .count {
    width: 150px;
    display: grid;
    gap: 5px;
    font-family: var(--mono);
    font-size: 13px;
  }
  .count .of {
    color: var(--faint);
  }
  .short {
    color: var(--amber);
  }
  .enough {
    color: var(--accent);
  }
  .count-bar {
    height: 5px;
    border-radius: 3px;
    background: var(--border);
    overflow: hidden;
  }
  .count-bar div {
    height: 100%;
    background: var(--amber);
  }
  .count-bar.enough div {
    background: var(--accent);
  }
  .save {
    display: flex;
    align-items: center;
    gap: 6px;
    padding-right: 8px;
    font-size: 13px;
    color: var(--faint);
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 4px;
    background: var(--accent);
  }

  .body-grid {
    display: grid;
    grid-template-columns: repeat(12, minmax(0, 1fr));
    gap: 20px;
    align-items: start;
  }
  .main-col {
    grid-column: span 8;
    min-width: 0;
  }
  .rail {
    grid-column: span 4;
    min-width: 0;
  }
  @media (max-width: 1100px) {
    .main-col,
    .rail {
      grid-column: 1 / -1;
    }
  }
  .essay-card {
    margin-bottom: 20px;
    padding: 36px 44px;
    border-radius: var(--radius-lg);
    background: var(--surface);
    border: 1px solid var(--border);
  }
  .essay {
    display: block;
    min-height: 60vh;
    padding: 0;
    border: none;
    background: transparent;
    font-size: 18px;
    line-height: 1.8;
    color: #e4e1d9;
    resize: vertical;
  }
  .essay:focus {
    box-shadow: none;
  }
  .essay-foot {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    margin-top: 12px;
    padding-top: 12px;
    border-top: 1px solid var(--line);
  }
  .main-col h2 {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .feedback-panel {
    display: grid;
    gap: 0;
  }
  .feedback {
    min-height: 180px;
    line-height: 1.6;
    font-size: 15px;
    field-sizing: content;
  }
  .link {
    font-weight: 600;
    text-decoration: none;
  }

  .compare-head {
    gap: 14px;
    margin-bottom: 16px;
    color: var(--faint);
  }
  .compare-head h2 {
    margin: 0 auto 0 0;
    color: var(--text);
  }
  .pick {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 8px;
  }
  .pick select {
    font-family: var(--mono);
    font-weight: 600;
  }

  @media (max-width: 700px) {
    .essay-card {
      padding: 20px 18px;
    }
    .tool-right {
      margin-left: 0;
      width: 100%;
      justify-content: space-between;
      gap: 12px;
    }
  }
</style>
