<script lang="ts">
  import { onMount } from "svelte";
  import { libraryInfo, revealLibrary, type LibraryInfo } from "./api";
  import type { Theme } from "./theme";

  let { theme = $bindable() }: { theme: Theme } = $props();
  let info: LibraryInfo | null = $state(null);
  let error = $state("");

  onMount(async () => {
    try {
      info = await libraryInfo();
    } catch (e) {
      error = String(e);
    }
  });

  const themes: { id: Theme; label: string }[] = [
    { id: "system", label: "Match system" },
    { id: "light", label: "Light" },
    { id: "dark", label: "Dark" },
  ];
</script>

<h1>Settings</h1>

<section class="card">
  <h2>Appearance</h2>
  <div class="seg">
    {#each themes as t}
      <button class:on={theme === t.id} onclick={() => (theme = t.id)}>{t.label}</button>
    {/each}
  </div>
</section>

<section class="card">
  <h2>Library</h2>
  {#if error}
    <p class="err">Couldn't open the library: {error}</p>
  {:else if info}
    <p class="path">{info.path}</p>
    <p class="muted">{info.recipe_count} recipes</p>
    <button class="link" onclick={revealLibrary}>Show in folder</button>
  {:else}
    <p class="muted">Loading…</p>
  {/if}
</section>

<style>
  h1 { font-family: var(--font-display); font-size: 30px; margin: 0 0 18px; }
  .card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 18px 20px;
    margin-bottom: 16px;
    max-width: 560px;
    box-shadow: var(--shadow);
  }
  h2 { font-size: 15px; margin: 0 0 12px; }
  .seg {
    display: inline-flex;
    background: var(--surface-2);
    border-radius: var(--radius-sm);
    padding: 3px;
  }
  .seg button {
    border: 0;
    background: transparent;
    padding: 6px 14px;
    border-radius: 7px;
    color: var(--text-2);
  }
  .seg button.on { background: var(--surface); color: var(--text); box-shadow: var(--shadow); }
  .path { margin: 0 0 4px; font-family: ui-monospace, Menlo, Consolas, monospace; font-size: 12.5px; user-select: text; word-break: break-all; }
  .muted { color: var(--text-2); margin: 0 0 10px; }
  .err { color: var(--accent); margin: 0; }
  .link { border: 0; background: none; padding: 0; color: var(--accent); font-weight: 600; }
</style>
