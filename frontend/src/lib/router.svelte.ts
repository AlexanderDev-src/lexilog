// A tiny hash router: "#/words/12" -> path "/words/12".
// Hash URLs need no server config, so refreshing any page just works.
// The file ends in .svelte.ts so it can use Svelte's $state rune.

function currentPath(): string {
  return location.hash.slice(1) || '/';
}

let path = $state(currentPath());

window.addEventListener('hashchange', () => {
  path = currentPath();
});

export const router = {
  get path() {
    return path;
  },
};

export function navigate(to: string) {
  location.hash = to;
}

/**
 * Matches "/words/:id" against "/words/12" and returns { id: "12" },
 * or null when the path doesn't fit the pattern.
 */
export function matchPath(pattern: string, actual: string): Record<string, string> | null {
  const want = pattern.split('/');
  const got = actual.split('?')[0].split('/');
  if (want.length !== got.length) return null;

  const params: Record<string, string> = {};
  for (let i = 0; i < want.length; i++) {
    if (want[i].startsWith(':')) params[want[i].slice(1)] = decodeURIComponent(got[i]);
    else if (want[i] !== got[i]) return null;
  }
  return params;
}
