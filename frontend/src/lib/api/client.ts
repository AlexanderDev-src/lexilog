// Thin wrapper around fetch for the JSON API.

export class ApiError extends Error {
  status: number;

  constructor(status: number, message: string) {
    super(message);
    this.status = status;
  }
}

async function request<T>(method: string, path: string, body?: unknown): Promise<T> {
  // A Blob (an image) is sent as raw bytes with its own type; anything else as JSON.
  const headers =
    body === undefined ? undefined : { 'content-type': body instanceof Blob ? body.type : 'application/json' };
  const res = await fetch(`/api${path}`, {
    method,
    headers,
    body: body === undefined ? undefined : body instanceof Blob ? body : JSON.stringify(body),
  });

  if (!res.ok) {
    // Our errors are {"error": "..."}; bad-JSON errors from the framework are plain text.
    const text = await res.text();
    let message = text || res.statusText;
    try {
      message = JSON.parse(text).error ?? message;
    } catch {
      // not JSON, keep the text
    }
    throw new ApiError(res.status, message);
  }

  if (res.status === 204) return undefined as T;
  return res.json() as Promise<T>;
}

export const http = {
  get: <T>(path: string) => request<T>('GET', path),
  post: <T>(path: string, body?: unknown) => request<T>('POST', path, body ?? {}),
  put: <T>(path: string, body: unknown) => request<T>('PUT', path, body),
  del: (path: string) => request<void>('DELETE', path),
};

/** Builds "?a=1&b=x", skipping empty values. */
export function query(params: Record<string, string | number | undefined | null>): string {
  const search = new URLSearchParams();
  for (const [key, value] of Object.entries(params)) {
    if (value !== undefined && value !== null && value !== '') search.set(key, String(value));
  }
  const text = search.toString();
  return text ? `?${text}` : '';
}
