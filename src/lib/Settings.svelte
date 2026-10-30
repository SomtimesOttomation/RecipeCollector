<script lang="ts">
  import { onMount } from "svelte";
  import { libraryInfo, revealLibrary, appVersion, autoCheckEnabled, setAutoCheck, openLink, type LibraryInfo } from "./api";
  import { updates, runCheck, runInstall } from "./updates.svelte";
  import type { Theme } from "./theme";

  let { theme = $bindable() }: { theme: Theme } = $props();
  let info: LibraryInfo | null = $state(null);
  let error = $state("");
  let version = $state("");
  let autoCheck = $state(autoCheckEnabled());

  $effect(() => setAutoCheck(autoCheck));

  onMount(async () => {
    version = await appVersion();
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

<section class="card">
  <h2>Updates</h2>
  <p class="muted">You have version {version}.</p>
  {#if updates.status?.state === "available"}
    {@const s = updates.status}
    <p><strong>Version {s.latest} is available.</strong></p>
    {#if s.notes}<pre class="notes">{s.notes}</pre>{/if}
    <div class="row">
      {#if s.installer_url}
        <button class="primary" disabled={updates.installing} onclick={runInstall}>
          {updates.installing ? "Downloading…" : "Install update"}
        </button>
      {/if}
      <button class="link" onclick={() => openLink(s.page_url)}>Open release page</button>
    </div>
  {:else if updates.status?.state === "up_to_date"}
    <p class="muted">You're up to date.</p>
  {:else if updates.status?.state === "unavailable"}
    <p class="muted">{updates.status.reason}</p>
  {/if}
  {#if updates.error}<p class="err">{updates.error}</p>{/if}
  <div class="row">
    <button class="secondary" disabled={updates.checking} onclick={runCheck}>
      {updates.checking ? "Checking…" : "Check for updates"}
    </button>
    <label class="check"><input type="checkbox" bind:checked={autoCheck} /> Check when the app starts</label>
  </div>
</section>

<style>
  .row { display: flex; align-items: center; gap: 14px; flex-wrap: wrap; margin-top: 10px; }
  .primary, .secondary {
    border: 0; border-radius: var(--radius-sm); padding: 7px 14px; font-weight: 600;
  }
  .primary { background: var(--accent); color: #fff; }
  .secondary { background: var(--surface-2); }
  .primary:disabled, .secondary:disabled { opacity: 0.6; cursor: default; }
  .check { display: inline-flex; gap: 6px; align-items: center; color: var(--text-2); }
  .notes {
    white-space: pre-wrap; font: inherit; font-size: 13px; color: var(--text-2);
    background: var(--surface-2); border-radius: var(--radius-sm); padding: 10px 12px; margin: 8px 0 0;
    max-height: 200px; overflow: auto; user-select: text;
  }
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
