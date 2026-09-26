<script lang="ts">
  import { onMount } from 'svelte';
  import { writingApi } from '../api/writing';
  import { estimateTokens, NORMAL_SIDE, prepareChart, type PreparedImage } from '../chartImage';
  import type { PieceImage } from '../types';
  import Icon from './Icon.svelte';

  // The Task 1 chart: paste it (Ctrl+V anywhere on the page), drop it here or
  // pick a file. It is shrunk in the browser, previewed with its token cost,
  // and only uploaded after "Attach".
  interface Props {
    pieceId: number;
    image: PieceImage | null;
    /** Called with the stored image after an upload, or null after removing it. */
    onchange: (image: PieceImage | null) => void;
  }

  let { pieceId, image, onchange }: Props = $props();

  // A processed image waiting for "Attach", plus the original for re-processing.
  let pending = $state<{ prepared: PreparedImage; url: string; original: Blob } | null>(null);
  let highDetail = $state(false);
  let busy = $state(false);
  let dragging = $state(false);
  let error = $state('');
  let fileInput: HTMLInputElement | undefined = $state();
  let dialog: HTMLDialogElement | undefined = $state();

  const src = $derived(image ? writingApi.imageUrl(pieceId, image.created_at) : '');

  onMount(() => {
    window.addEventListener('paste', onPaste);
    return () => {
      window.removeEventListener('paste', onPaste);
      if (pending) URL.revokeObjectURL(pending.url);
    };
  });

  /**
   * Takes a pasted image, unless the paste is text going into a text box
   * (copying from Word or Excel puts both text and a picture on the clipboard).
   */
  function onPaste(event: ClipboardEvent) {
    const file = [...(event.clipboardData?.files ?? [])].find((f) => f.type.startsWith('image/'));
    if (!file) return;
    const target = event.target as HTMLElement | null;
    const intoTextBox = target?.closest('textarea, input, [contenteditable]') !== null;
    if (intoTextBox && event.clipboardData?.types.includes('text/plain')) return;
    event.preventDefault();
    load(file);
  }

  function onDrop(event: DragEvent) {
    event.preventDefault();
    dragging = false;
    const file = [...(event.dataTransfer?.files ?? [])].find((f) => f.type.startsWith('image/'));
    if (file) load(file);
    else error = 'Drop an image file (PNG, JPEG or WebP).';
  }

  function onPick() {
    const file = fileInput?.files?.[0];
    if (file) load(file);
    if (fileInput) fileInput.value = ''; // picking the same file again still fires
  }

  async function load(original: Blob, detail = highDetail) {
    busy = true;
    error = '';
    try {
      const prepared = await prepareChart(original, detail);
      if (pending) URL.revokeObjectURL(pending.url);
      pending = { prepared, url: URL.createObjectURL(prepared.blob), original };
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      busy = false;
    }
  }

  function toggleDetail() {
    highDetail = !highDetail;
    if (pending) load(pending.original, highDetail);
  }

  function cancel() {
    if (pending) URL.revokeObjectURL(pending.url);
    pending = null;
  }

  async function attach() {
    if (!pending) return;
    busy = true;
    error = '';
    try {
      const piece = await writingApi.putImage(pieceId, pending.prepared.blob);
      cancel();
      onchange(piece.image);
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      busy = false;
    }
  }

  async function remove() {
    if (!confirm('Remove the chart from this piece?')) return;
    try {
      await writingApi.removeImage(pieceId);
      onchange(null);
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    }
  }

  function kb(bytes: number): string {
    return bytes < 1024 * 1024 ? `${Math.round(bytes / 1024)} KB` : `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  }

  function format(mime: string): string {
    return mime.replace('image/', '').toUpperCase();
  }
</script>

<div
  class="chart"
  class:dragging
  role="group"
  aria-label="Chart image"
  ondragover={(e) => {
    e.preventDefault();
    dragging = true;
  }}
  ondragleave={() => (dragging = false)}
  ondrop={onDrop}
>
  {#if pending}
    {@const p = pending.prepared}
    <img class="thumb" src={pending.url} alt="Preview of the chart to attach" />
    <p class="info">
      {p.width} × {p.height} px · {format(p.blob.type)} {kb(p.blob.size)} · ≈ {estimateTokens(p.width, p.height).toLocaleString()} tokens
    </p>
    <p class="muted small">
      From {p.sourceWidth} × {p.sourceHeight}{p.cropped ? ', margins cropped' : ''}{p.resized ? ', shrunk and sharpened' : ''}.
    </p>
    {#if Math.max(p.sourceWidth, p.sourceHeight) > NORMAL_SIDE}
      <label class="check">
        <input type="checkbox" checked={highDetail} onchange={toggleDetail} disabled={busy} />
        High detail (1280 px): for dense charts with many small numbers
      </label>
    {/if}
    <div class="row">
      <button type="button" class="primary" onclick={attach} disabled={busy}>{busy ? 'Working…' : 'Attach chart'}</button>
      <button type="button" class="ghost" onclick={cancel} disabled={busy}>Cancel</button>
    </div>
  {:else if image}
    <button type="button" class="thumb-btn" onclick={() => dialog?.showModal()} title="Show the chart full size">
      <img class="thumb" {src} alt="The task's chart" />
    </button>
    <div class="row spread wrap">
      <span class="info">{image.width} × {image.height} · ≈ {estimateTokens(image.width, image.height).toLocaleString()} tokens</span>
      <span class="row actions">
        <button type="button" class="ghost small-btn" onclick={() => fileInput?.click()}>Replace</button>
        <button type="button" class="ghost small-btn danger-text" onclick={remove}>Remove</button>
      </span>
    </div>
  {:else}
    <button type="button" class="drop" onclick={() => fileInput?.click()} disabled={busy}>
      <Icon name="chart" size={22} />
      <span class="drop-title">{busy ? 'Preparing…' : 'Add the chart'}</span>
      <span class="muted small">Paste it (Ctrl+V), drop it here, or click to choose a file</span>
    </button>
  {/if}
  {#if error}<p class="error small">{error}</p>{/if}
  <input bind:this={fileInput} class="visually-hidden" type="file" accept="image/*" onchange={onPick} tabindex="-1" aria-hidden="true" />
</div>

{#if image}
  <!-- Full-size view. Clicking anywhere or pressing Esc closes it. -->
  <dialog bind:this={dialog} class="lightbox" onclick={() => dialog?.close()}>
    <img {src} alt="The task's chart, full size" />
  </dialog>
{/if}

<style>
  .chart {
    display: grid;
    gap: 8px;
    padding: 10px;
    border: 1px solid var(--border);
    border-radius: 16px;
    background: var(--surface);
  }
  .chart.dragging {
    border-color: var(--accent);
    background: var(--accent-soft);
  }
  .drop {
    display: grid;
    justify-items: center;
    gap: 4px;
    min-height: 130px;
    padding: 16px;
    border: 1px dashed #3a3e49;
    border-radius: 12px;
    font-weight: 400;
    text-align: center;
    color: var(--muted);
  }
  .drop-title {
    font-weight: 600;
    color: var(--text);
  }
  .thumb-btn {
    display: block;
    min-height: 0;
    padding: 0;
    border: none;
    background: none;
    cursor: zoom-in;
  }
  .thumb-btn:hover:not(:disabled) {
    background: none;
  }
  .thumb {
    display: block;
    width: 100%;
    max-height: 260px;
    object-fit: contain;
    border-radius: 8px;
    background: #fff;
  }
  .info {
    margin: 0;
    font-family: var(--mono);
    font-size: 12px;
    color: var(--faint);
  }
  .chart p {
    margin: 0;
  }
  .actions {
    gap: 4px;
  }
  .check {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    font-size: 0.8rem;
  }
  .row .primary {
    min-height: 40px;
  }
  .small-btn {
    min-height: 32px;
    padding: 0 10px;
    font-size: 13px;
  }
  .lightbox {
    max-width: 95vw;
    max-height: 95vh;
    padding: 0;
    border: none;
    border-radius: 12px;
    background: #fff;
    cursor: zoom-out;
  }
  .lightbox::backdrop {
    background: rgba(0, 0, 0, 0.75);
  }
  .lightbox img {
    display: block;
    max-width: 95vw;
    max-height: 95vh;
  }
</style>
