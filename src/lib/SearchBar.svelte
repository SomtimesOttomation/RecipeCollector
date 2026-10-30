<script lang="ts">
  import { lib, search, searchActive, queueSearch, clearSearch } from "./store.svelte";

  let authors = $derived([...new Set(lib.recipes.map((r) => r.author).filter((a): a is string => !!a))].sort((a, b) => a.localeCompare(b)));
  let sites = $derived([...new Set(lib.recipes.map((r) => r.site_name).filter((s): s is string => !!s))].sort((a, b) => a.localeCompare(b)));
  const times = [0, 15, 30, 45, 60, 90, 120];

  function toggleTag(name: string) {
    const i = search.tags.findIndex((t) => t.toLowerCase() === name.toLowerCase());
    if (i >= 0) search.tags.splice(i, 1);
    else search.tags.push(name);
  }
  function hasTag(name: string) {
    return search.tags.some((t) => t.toLowerCase() === name.toLowerCase());
  }
  function folderValue() {
    return typeof search.folder === "number" ? String(search.folder) : search.folder;
  }
  function setFolder(v: string) {
    search.folder = v === "any" || v === "none" ? v : parseInt(v, 10);
  }
  function key(e: KeyboardEvent) {
    if (e.key === "Escape") clearSearch();
  }
</script>

<div class="search">
  <div class="field">
    <span class="glass">⌕</span>
    <input
      type="search"
      placeholder="Search recipes, ingredients, notes…"
      bind:value={search.query}
      oninput={queueSearch}
      onkeydown={key}
    />
    {#if searchActive()}
      <button class="clear" title="Clear search" onclick={clearSearch}>✕</button>
    {/if}
  </div>
  <button class="adv" class:on={search.advanced} onclick={() => (search.advanced = !search.advanced)}>
    Advanced{#if search.tags.length || search.author || search.site || search.maxTime || search.folder !== "any"}
      <span class="dot"></span>{/if}
  </button>
</div>

{#if search.advanced}
  <div class="panel">
    <div class="filters">
      <label>Author
        <select bind:value={search.author}>
          <option value="">Any</option>
          {#each authors as a}<option value={a}>{a}</option>{/each}
        </select>
      </label>
      <label>Site
        <select bind:value={search.site}>
          <option value="">Any</option>
          {#each sites as s}<option value={s}>{s}</option>{/each}
        </select>
      </label>
      <label>Total time
        <select bind:value={search.maxTime}>
          {#each times as t}<option value={t}>{t === 0 ? "Any" : `Up to ${t < 60 ? t + " min" : t / 60 + " h"}`}</option>{/each}
        </select>
      </label>
      <label>Folder
        <select value={folderValue()} onchange={(e) => setFolder((e.target as HTMLSelectElement).value)}>
          <option value="any">Any</option>
          <option value="none">Not in a folder</option>
          {#each lib.folders as f (f.id)}<option value={String(f.id)}>{f.name}</option>{/each}
        </select>
      </label>
    </div>
    {#if lib.tags.length}
      <div class="tag-filter">
        <span class="lbl">Tags <small>(type, cuisine, diet… recipes must have all selected)</small></span>
        <div class="chips">
          {#each lib.tags as t (t.id)}
            <button class:on={hasTag(t.name)} onclick={() => toggleTag(t.name)}>#{t.name}</button>
          {/each}
        </div>
      </div>
    {/if}
  </div>
{/if}

<style>
  .search { display: flex; gap: 10px; align-items: center; max-width: 720px; margin-bottom: 14px; }
  .field { position: relative; flex: 1; display: flex; align-items: center; }
  .glass { position: absolute; left: 12px; color: var(--text-2); font-size: 16px; pointer-events: none; }
  input {
    width: 100%; padding: 9px 34px; border-radius: var(--radius-sm); border: 1px solid var(--border);
    background: var(--surface-2); outline: none; font: inherit; user-select: text; color: var(--text);
  }
  input:focus { border-color: var(--accent); background: var(--surface); }
  input::-webkit-search-cancel-button { display: none; }
  .clear { position: absolute; right: 8px; border: 0; background: none; color: var(--text-2); padding: 4px; }
  .adv {
    position: relative; border: 1px solid var(--border); background: var(--surface); border-radius: var(--radius-sm);
    padding: 8px 14px; font-weight: 600; color: var(--text-2);
  }
  .adv.on { border-color: var(--accent); color: var(--accent); }
  .dot { position: absolute; top: 5px; right: 5px; width: 7px; height: 7px; border-radius: 50%; background: var(--accent); }
  .panel {
    max-width: 720px; margin: -4px 0 18px; padding: 14px 16px; border-radius: var(--radius);
    background: var(--surface); border: 1px solid var(--border); box-shadow: var(--shadow);
  }
  .filters { display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 12px; }
  label { display: flex; flex-direction: column; gap: 4px; font-size: 12px; font-weight: 600; color: var(--text-2); }
  select {
    font: inherit; font-size: 13.5px; font-weight: 400; color: var(--text); padding: 6px 8px;
    border-radius: 7px; border: 1px solid var(--border); background: var(--surface-2);
  }
  .tag-filter { margin-top: 14px; }
  .lbl { font-size: 12px; font-weight: 600; color: var(--text-2); }
  .lbl small { font-weight: 400; }
  .chips { display: flex; flex-wrap: wrap; gap: 6px; margin-top: 6px; }
  .chips button {
    border: 1px solid var(--border); background: none; border-radius: 99px; padding: 3px 10px; font-size: 12.5px; color: var(--text-2);
  }
  .chips button.on { background: var(--accent-soft); border-color: var(--accent); color: var(--accent); font-weight: 600; }
</style>
