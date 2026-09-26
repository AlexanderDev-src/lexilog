<script lang="ts" module>
  // Stroke icons drawn with the text colour (currentColor). Each icon is a
  // list of SVG path strings on a 24x24 grid.
  const PATHS = {
    today: [
      'M6 4h12a3 3 0 0 1 3 3v11a3 3 0 0 1-3 3H6a3 3 0 0 1-3-3V7a3 3 0 0 1 3-3Z',
      'M3 9h18M8 2v4M16 2v4',
    ],
    words: ['M12 3 3 8l9 5 9-5-9-5Z', 'm3 13 9 5 9-5'],
    review: ['M3 12a9 9 0 0 1 15.5-6.2L21 8', 'M21 3v5h-5', 'M21 12a9 9 0 0 1-15.5 6.2L3 16', 'M3 21v-5h5'],
    writing: ['M4 20h4L19 9l-4-4L4 16v4Z', 'm13.5 6.5 4 4'],
    practice: [
      'M4 15v-3a8 8 0 0 1 16 0v3',
      'M4.5 14h1A1.5 1.5 0 0 1 7 15.5v3A1.5 1.5 0 0 1 5.5 20h-1A1.5 1.5 0 0 1 3 18.5v-3A1.5 1.5 0 0 1 4.5 14Z',
      'M18.5 14h1a1.5 1.5 0 0 1 1.5 1.5v3a1.5 1.5 0 0 1-1.5 1.5h-1a1.5 1.5 0 0 1-1.5-1.5v-3a1.5 1.5 0 0 1 1.5-1.5Z',
    ],
    mistakes: ['M5 21V4h12l-2 4 2 4H5'],
    plus: ['M12 5v14M5 12h14'],
    search: ['M18 11a7 7 0 1 1-14 0a7 7 0 0 1 14 0Z', 'm20 20-3.5-3.5'],
    'arrow-right': ['M5 12h14M13 6l6 6-6 6'],
    'arrow-left': ['M19 12H5M11 6l-6 6 6 6'],
    clock: ['M20 13a8 8 0 1 1-16 0a8 8 0 0 1 16 0Z', 'M12 9v4l2 2M10 2h4'],
    check: ['m5 12 5 5 9-10'],
    close: ['M6 6l12 12M18 6 6 18'],
    compare: [
      'M5 4h3a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2Z',
      'M16 4h3a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2h-3a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2Z',
    ],
    chart: ['M3 20h18', 'M6 16v-5', 'M11 16V6', 'M16 16v-8'],
    sparkle: ['M12 3v4M12 17v4M3 12h4M17 12h4', 'm6 6 2.5 2.5M15.5 15.5 18 18M18 6l-2.5 2.5M8.5 15.5 6 18'],
  } as const;

  export type IconName = keyof typeof PATHS;
</script>

<script lang="ts">
  let { name, size = 20, stroke = 1.8 }: { name: IconName; size?: number; stroke?: number } = $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width={stroke}
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  {#each PATHS[name] as d (d)}<path {d} />{/each}
</svg>
