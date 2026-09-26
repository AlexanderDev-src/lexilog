// Prepares a Task 1 chart for upload, in the browser:
//   1. flatten onto white (a transparent PNG would look black to some models)
//   2. crop plain margins of one colour
//   3. shrink by halving until close to the target, then one last exact step
//      (a single big jump skips pixels and makes thin lines and text ragged)
//   4. if it was shrunk, sharpen lightly (an "unsharp mask")
//   5. save as PNG, or as WebP when the PNG is too big
// Never enlarges: a small image is kept at its size.
// Tests on the university gateway: 1024 px read a dense line chart almost as
// well as 1280 px (32/36 values vs 33/36) for ~40% fewer tokens.

/** Longest side after shrinking. */
export const NORMAL_SIDE = 1024;
export const HIGH_DETAIL_SIDE = 1280;
/** Above this, try WebP instead of PNG. */
const PNG_LIMIT = 1024 * 1024;

export interface PreparedImage {
  blob: Blob;
  width: number;
  height: number;
  /** Size of the file that was pasted or dropped. */
  sourceWidth: number;
  sourceHeight: number;
  cropped: boolean;
  resized: boolean;
}

/** Rough input tokens for an image on the gateway: width × height / 750. */
export function estimateTokens(width: number, height: number): number {
  return Math.ceil((width * height) / 750);
}

export async function prepareChart(file: Blob, highDetail: boolean): Promise<PreparedImage> {
  let bitmap: ImageBitmap;
  try {
    bitmap = await createImageBitmap(file);
  } catch {
    throw new Error('This file could not be read as an image. Try a PNG, JPEG or WebP.');
  }
  const sourceWidth = bitmap.width;
  const sourceHeight = bitmap.height;

  // 1. Flatten onto white.
  let canvas = makeCanvas(sourceWidth, sourceHeight);
  const ctx = context(canvas);
  ctx.fillStyle = '#fff';
  ctx.fillRect(0, 0, sourceWidth, sourceHeight);
  ctx.drawImage(bitmap, 0, 0);
  bitmap.close();

  // 2. Crop margins.
  const box = contentBox(ctx.getImageData(0, 0, sourceWidth, sourceHeight));
  const cropped = box.w < sourceWidth || box.h < sourceHeight;
  if (cropped) {
    const next = makeCanvas(box.w, box.h);
    context(next).drawImage(canvas, box.x, box.y, box.w, box.h, 0, 0, box.w, box.h);
    canvas = next;
  }

  // 3. Shrink.
  const target = highDetail ? HIGH_DETAIL_SIDE : NORMAL_SIDE;
  const resized = Math.max(canvas.width, canvas.height) > target;
  if (resized) {
    while (Math.max(canvas.width, canvas.height) / 2 >= target) {
      canvas = scaled(canvas, Math.round(canvas.width / 2), Math.round(canvas.height / 2));
    }
    const scale = target / Math.max(canvas.width, canvas.height);
    if (scale < 1) {
      canvas = scaled(canvas, Math.round(canvas.width * scale), Math.round(canvas.height * scale));
    }

    // 4. Sharpen what the shrinking softened.
    const c = context(canvas);
    const image = c.getImageData(0, 0, canvas.width, canvas.height);
    unsharpMask(image, 0.5);
    c.putImageData(image, 0, 0);
  }

  // 5. Save.
  let blob = await toBlob(canvas, 'image/png');
  if (blob.size > PNG_LIMIT) {
    const webp = await toBlob(canvas, 'image/webp', 0.92).catch(() => null);
    // Some browsers ignore the WebP request and return PNG again.
    if (webp && webp.type === 'image/webp' && webp.size < blob.size) blob = webp;
  }

  return { blob, width: canvas.width, height: canvas.height, sourceWidth, sourceHeight, cropped, resized };
}

function makeCanvas(width: number, height: number): HTMLCanvasElement {
  const canvas = document.createElement('canvas');
  canvas.width = width;
  canvas.height = height;
  return canvas;
}

function context(canvas: HTMLCanvasElement): CanvasRenderingContext2D {
  const ctx = canvas.getContext('2d', { willReadFrequently: true });
  if (!ctx) throw new Error('This browser cannot process images (no canvas).');
  return ctx;
}

function scaled(source: HTMLCanvasElement, width: number, height: number): HTMLCanvasElement {
  const canvas = makeCanvas(width, height);
  const ctx = context(canvas);
  ctx.imageSmoothingEnabled = true;
  ctx.imageSmoothingQuality = 'high';
  ctx.drawImage(source, 0, 0, width, height);
  return canvas;
}

function toBlob(canvas: HTMLCanvasElement, type: string, quality?: number): Promise<Blob> {
  return new Promise((resolve, reject) =>
    canvas.toBlob((blob) => (blob ? resolve(blob) : reject(new Error('Could not save the image.'))), type, quality),
  );
}

/**
 * The part of the image inside plain margins. The margin colour is the
 * top-left pixel; a row or column belongs to the margin when every pixel in
 * it is within a small tolerance of that colour (JPEG noise, faint shadows).
 * A few pixels of margin are kept so text doesn't touch the edge.
 */
function contentBox(image: ImageData): { x: number; y: number; w: number; h: number } {
  const { width, height, data } = image;
  const TOLERANCE = 16;
  const KEEP = 8;
  const bg = [data[0], data[1], data[2]];
  const isBg = (x: number, y: number) => {
    const i = (y * width + x) * 4;
    return (
      Math.abs(data[i] - bg[0]) <= TOLERANCE &&
      Math.abs(data[i + 1] - bg[1]) <= TOLERANCE &&
      Math.abs(data[i + 2] - bg[2]) <= TOLERANCE
    );
  };
  const rowIsBg = (y: number) => {
    for (let x = 0; x < width; x++) if (!isBg(x, y)) return false;
    return true;
  };
  const colIsBg = (x: number, top: number, bottom: number) => {
    for (let y = top; y <= bottom; y++) if (!isBg(x, y)) return false;
    return true;
  };

  let top = 0;
  while (top < height && rowIsBg(top)) top++;
  if (top === height) return { x: 0, y: 0, w: width, h: height }; // one colour: nothing to crop
  let bottom = height - 1;
  while (bottom > top && rowIsBg(bottom)) bottom--;
  let left = 0;
  while (left < width && colIsBg(left, top, bottom)) left++;
  let right = width - 1;
  while (right > left && colIsBg(right, top, bottom)) right--;

  const x = Math.max(0, left - KEEP);
  const y = Math.max(0, top - KEEP);
  return { x, y, w: Math.min(width, right + KEEP + 1) - x, h: Math.min(height, bottom + KEEP + 1) - y };
}

/**
 * Sharpening: result = original + amount × (original − blurred).
 * The blur is a small [1 2 1] kernel, run across and then down.
 */
function unsharpMask(image: ImageData, amount: number) {
  const { width, height, data } = image;
  const blurred = new Float32Array(data.length);
  const tmp = new Float32Array(data.length);
  const at = (x: number, y: number) => (y * width + x) * 4;

  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const l = at(Math.max(0, x - 1), y);
      const c = at(x, y);
      const r = at(Math.min(width - 1, x + 1), y);
      for (let ch = 0; ch < 3; ch++) tmp[c + ch] = (data[l + ch] + 2 * data[c + ch] + data[r + ch]) / 4;
    }
  }
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const u = at(x, Math.max(0, y - 1));
      const c = at(x, y);
      const d = at(x, Math.min(height - 1, y + 1));
      for (let ch = 0; ch < 3; ch++) blurred[c + ch] = (tmp[u + ch] + 2 * tmp[c + ch] + tmp[d + ch]) / 4;
    }
  }
  for (let i = 0; i < data.length; i += 4) {
    for (let ch = 0; ch < 3; ch++) {
      // Uint8ClampedArray keeps the result within 0-255 by itself.
      data[i + ch] = data[i + ch] + amount * (data[i + ch] - blurred[i + ch]);
    }
  }
}
