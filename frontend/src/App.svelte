<script lang="ts">
  import type { Component } from 'svelte';
  import { cardsApi } from './lib/api/cards';
  import Icon, { type IconName } from './lib/components/Icon.svelte';
  import { matchPath, router } from './lib/router.svelte';
  import CardEdit from './pages/CardEdit.svelte';
  import Home from './pages/Home.svelte';
  import Mistakes from './pages/Mistakes.svelte';
  import Practice from './pages/Practice.svelte';
  import Review from './pages/Review.svelte';
  import Vocabulary from './pages/Vocabulary.svelte';
  import WritingEditor from './pages/WritingEditor.svelte';
  import WritingList from './pages/WritingList.svelte';

  const routes: { pattern: string; page: Component<any> }[] = [
    { pattern: '/', page: Home },
    { pattern: '/words', page: Vocabulary },
    { pattern: '/words/:id', page: CardEdit },
    { pattern: '/review', page: Review },
    { pattern: '/writing', page: WritingList },
    { pattern: '/writing/:id', page: WritingEditor },
    { pattern: '/practice', page: Practice },
    { pattern: '/mistakes', page: Mistakes },
  ];

  const NAV: { href: string; label: string; section: string; icon: IconName }[] = [
    { href: '#/', label: 'Today', section: '/', icon: 'today' },
    { href: '#/words', label: 'Vocabulary', section: '/words', icon: 'words' },
    { href: '#/review', label: 'Review', section: '/review', icon: 'review' },
    { href: '#/writing', label: 'Writing', section: '/writing', icon: 'writing' },
    { href: '#/practice', label: 'Practice log', section: '/practice', icon: 'practice' },
    { href: '#/mistakes', label: 'Mistakes', section: '/mistakes', icon: 'mistakes' },
  ];

  // First route whose pattern fits the current path, plus its :params.
  const current = $derived.by(() => {
    for (const route of routes) {
      const params = matchPath(route.pattern, router.path);
      if (params) return { page: route.page, params };
    }
    return null;
  });

  // Review runs full screen, without the sidebar, so nothing distracts.
  const focusMode = $derived(router.path === '/review');

  function isActive(section: string): boolean {
    if (section === '/') return router.path === '/';
    return router.path === section || router.path.startsWith(`${section}/`);
  }

  // Due count for the Review badge, refreshed on every page change.
  let dueCount = $state(0);
  $effect(() => {
    void router.path;
    cardsApi
      .due(1)
      .then((q) => (dueCount = q.total))
      .catch(() => {});
  });
</script>

<div class="shell" class:focus={focusMode}>
  {#if !focusMode}
    <aside class="sidebar">
      <a class="brand" href="#/">
        <span class="mark">Aa</span>
        <span class="brand-text">
          <span class="brand-name">IELTS Practice</span>
          <span class="brand-sub">Academic module</span>
        </span>
      </a>
      <nav aria-label="Main">
        {#each NAV as item (item.href)}
          <a
            href={item.href}
            class:active={isActive(item.section)}
            aria-current={isActive(item.section) ? 'page' : undefined}
          >
            <Icon name={item.icon} />
            <span class="nav-label">{item.label}</span>
            {#if item.section === '/review' && dueCount > 0}<span class="count">{dueCount}</span>{/if}
          </a>
        {/each}
      </nav>
      <div class="sidebar-foot">
        <a class="add-word" href="#/words"><Icon name="plus" size={18} stroke={2} /> Add a word</a>
        <span class="host">home server · :1111</span>
      </div>
    </aside>
  {/if}

  <main>
    <!-- {#key} remounts the page when the URL changes, so it loads fresh data. -->
    {#key router.path}
      {#if current}
        <current.page {...current.params} />
      {:else}
        <h1>Not found</h1>
        <p><a href="#/">Go to Today</a></p>
      {/if}
    {/key}
  </main>
</div>

<style>
  .shell {
    display: grid;
    grid-template-columns: 240px minmax(0, 1fr);
    min-height: 100vh;
  }
  .shell.focus {
    grid-template-columns: minmax(0, 1fr);
  }

  .sidebar {
    position: sticky;
    top: 0;
    height: 100vh;
    padding: 28px 16px 24px;
    background: var(--sidebar);
    border-right: 1px solid var(--line);
    display: flex;
    flex-direction: column;
    gap: 32px;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 0 8px;
    color: var(--text);
    text-decoration: none;
  }
  .brand:hover {
    color: var(--text);
  }
  .mark {
    width: 36px;
    height: 36px;
    flex-shrink: 0;
    border-radius: 10px;
    background: var(--accent);
    color: var(--on-accent);
    display: grid;
    place-items: center;
    font-family: var(--display);
    font-weight: 800;
    font-size: 16px;
  }
  .brand-text {
    display: grid;
  }
  .brand-name {
    font-family: var(--display);
    font-weight: 700;
    font-size: 17px;
  }
  .brand-sub {
    font-size: 12px;
    color: var(--faint);
  }

  nav {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  nav a {
    height: 44px;
    padding: 0 12px;
    border-radius: 10px;
    display: flex;
    align-items: center;
    gap: 12px;
    color: var(--muted);
    text-decoration: none;
    font-weight: 500;
    font-size: 15px;
  }
  nav a:hover {
    color: var(--text);
    background: var(--hover);
  }
  nav a.active {
    color: var(--accent);
    background: var(--accent-soft);
    font-weight: 600;
  }
  .nav-label {
    flex: 1;
  }
  .count {
    min-width: 28px;
    height: 22px;
    padding: 0 7px;
    border-radius: 11px;
    background: var(--accent);
    color: var(--on-accent);
    font-family: var(--mono);
    font-size: 12px;
    font-weight: 600;
    display: grid;
    place-items: center;
  }

  .sidebar-foot {
    margin-top: auto;
    display: grid;
    gap: 12px;
    padding: 0 8px;
  }
  .add-word {
    height: 44px;
    border-radius: 10px;
    border: 1px dashed #3a3e49;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    color: var(--text);
    text-decoration: none;
    font-weight: 600;
    font-size: 14px;
  }
  .add-word:hover {
    border-color: var(--accent);
    color: var(--accent);
  }
  .host {
    font-family: var(--mono);
    font-size: 11px;
    color: var(--faint);
  }

  main {
    width: 100%;
    max-width: var(--page-width);
    padding: 40px 56px 64px;
    min-width: 0;
  }
  .focus main {
    max-width: none;
    padding: 0;
  }

  /* Narrow screens: the sidebar becomes a top bar with a scrolling nav. */
  @media (max-width: 900px) {
    .shell {
      grid-template-columns: minmax(0, 1fr);
    }
    .sidebar {
      z-index: 10;
      height: auto;
      padding: 10px 12px;
      flex-direction: row;
      align-items: center;
      gap: 10px;
      border-right: none;
      border-bottom: 1px solid var(--line);
    }
    .brand {
      padding: 0;
    }
    .brand-text,
    .sidebar-foot {
      display: none;
    }
    nav {
      flex-direction: row;
      overflow-x: auto;
      scrollbar-width: none;
    }
    nav a {
      flex-shrink: 0;
      gap: 6px;
      padding: 0 10px;
      font-size: 14px;
    }
    main {
      padding: 20px 16px 64px;
    }
  }
</style>
