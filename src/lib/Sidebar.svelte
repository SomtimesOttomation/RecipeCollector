<script lang="ts">
  import { tick } from "svelte";
  import { createFolder, renameFolder, deleteFolder, confirmDeleteFolder, renameTag, deleteTag, confirmDeleteTag } from "./api";
  import { lib, refresh, inScope, moveTo, draggedRecipe, isRecipeDrag, RECIPE_DRAG, type Scope } from "./store.svelte";

  type View = "library" | "settings";
  let {
    view,
    openId,
    onnavigate,
    onopen,
  }: { view: View; openId: number | null; onnavigate: (v: View) => void; onopen: (id: number) => void } = $props();

  // Which groups are folded away; remembered between launches.
  const FOLD_KEY = "rc.sidebarFolded";
  let folded = $state<Record<string, boolean>>(loadFolded());
  function loadFolded(): Record<string, boolean> {
    try {
      return JSON.parse(localStorage.getItem(FOLD_KEY) ?? "{}");
    } catch {
      return {};
    }
  }
  function toggle(key: string) {
    folded[key] = !folded[key];
    try {
      localStorage.setItem(FOLD_KEY, JSON.stringify(folded));
    } catch {}
  }

  let dropTarget = $state<string | null>(null);
  let editingId = $state<number | null>(null);
  let editName = $state("");
  let menuFor = $state<number | null>(null);
  let adding = $state(false);
  let tagMenu = $state<number | null>(null);
  let editingTag = $state<number | null>(null);
  let tagName = $state("");

  async function startTagRename(id: number, name: string) {
    tagMenu = null;
    editingTag = id;
    tagName = name;
    await tick();
    document.querySelector<HTMLInputElement>(".rename-tag")?.select();
  }
  async function finishTagRename() {
    if (editingTag === null) return;
    const id = editingTag;
    const old = lib.tags.find((t) => t.id === id)?.name;
    editingTag = null;
    if (tagName.trim() && tagName.trim() !== old) {
      await renameTag(id, tagName);
      if (lib.scope.kind === "tag" && old && lib.scope.name.toLowerCase() === old.toLowerCase()) lib.scope = { kind: "tag", name: tagName.trim() };
      await refresh();
    }
  }
  async function removeTag(id: number, name: string) {
    tagMenu = null;
    if (await confirmDeleteTag(name)) {
      await deleteTag(id);
      await refresh();
    }
  }
  let newName = $state("");

  function select(s: Scope) {
    lib.scope = s;
    onnavigate("library");
  }
  function isActive(s: Scope) {
    if (view !== "library" || openId !== null) return false;
    const c = lib.scope;
    if (c.kind === "folder") return s.kind === "folder" && c.id === s.id;
    if (c.kind === "tag") return s.kind === "tag" && c.name.toLowerCase() === s.name.toLowerCase();
    return c.kind === s.kind;
  }

  // ---- drag and drop ----
  function over(e: DragEvent, key: string) {
    if (!isRecipeDrag(e)) return;
    e.preventDefault();
    e.stopPropagation();
    if (e.dataTransfer) e.dataTransfer.dropEffect = "move";
    dropTarget = key;
  }
  async function dropOn(e: DragEvent, folderId: number | null) {
    const id = draggedRecipe(e);
    dropTarget = null;
    if (id === null) return;
    e.preventDefault();
    e.stopPropagation();
    await moveTo(id, folderId);
  }
  function dragStart(e: DragEvent, id: number) {
    e.dataTransfer?.setData(RECIPE_DRAG, String(id));
    if (e.dataTransfer) e.dataTransfer.effectAllowed = "move";
  }

  // ---- folder editing ----
  async function startAdd() {
    adding = true;
    newName = "";
    await tick();
    document.querySelector<HTMLInputElement>(".new-folder input")?.focus();
  }
  async function finishAdd() {
    if (!adding) return;
    adding = false;
    if (newName.trim()) {
      const f = await createFolder(newName);
      await refresh();
      if (f) folded[`f${f.id}`] = false;
    }
  }
  async function startRename(id: number, name: string) {
    menuFor = null;
    editingId = id;
    editName = name;
    await tick();
    document.querySelector<HTMLInputElement>(".rename")?.select();
  }
  async function finishRename() {
    if (editingId === null) return;
    const id = editingId;
    editingId = null;
    if (editName.trim()) {
      await renameFolder(id, editName);
      await refresh();
    }
  }
  async function remove(id: number, name: string) {
    menuFor = null;
    if (await confirmDeleteFolder(name)) {
      await deleteFolder(id);
      await refresh();
    }
  }
  function keys(e: KeyboardEvent, done: () => void, cancel: () => void) {
    if (e.key === "Enter") done();
    if (e.key === "Escape") cancel();
  }

  let unfiled = $derived(lib.recipes.filter((r) => inScope(r, { kind: "unfiled" })));
</script>

<svelte:window onclick={() => { menuFor = null; tagMenu = null; }} ondragend={() => (dropTarget = null)} />

<nav>
  <div class="drag" data-tauri-drag-region></div>
  <div class="brand">
    <span class="logo">🍲</span>
    <span class="name">Recipe Collector</span>
  </div>

  <div class="scroll">
    <button class="row" class:active={isActive({ kind: "all" })} onclick={() => select({ kind: "all" })}>
      <span class="ico">▦</span><span class="label">All recipes</span><span class="count">{lib.recipes.length}</span>
    </button>

    {#if lib.folders.length > 0}
      <div class="group">
        <div
          class="row-wrap"
          class:drop={dropTarget === "unfiled"}
          role="group"
          ondragover={(e) => over(e, "unfiled")}
          ondragleave={() => (dropTarget = null)}
          ondrop={(e) => dropOn(e, null)}
        >
          <button class="chev" class:open={!folded.unfiled} onclick={() => toggle("unfiled")} aria-label="Show or hide">›</button>
          <button class="row" class:active={isActive({ kind: "unfiled" })} onclick={() => select({ kind: "unfiled" })}>
            <span class="label">Not in a folder</span><span class="count">{unfiled.length}</span>
          </button>
        </div>
        {#if !folded.unfiled}
          {@render items(unfiled)}
        {/if}
      </div>
    {/if}

    <div class="section">
      <span>Folders</span>
      <button class="plus" title="New folder" onclick={startAdd}>+</button>
    </div>

    {#each lib.folders as f (f.id)}
      {@const key = `f${f.id}`}
      <div class="group">
        <div
          class="row-wrap"
          class:drop={dropTarget === key}
          role="group"
          ondragover={(e) => over(e, key)}
          ondragleave={() => (dropTarget = null)}
          ondrop={(e) => dropOn(e, f.id)}
        >
          <button class="chev" class:open={!folded[key]} onclick={() => toggle(key)} aria-label="Show or hide">›</button>
          {#if editingId === f.id}
            <input
              class="rename"
              bind:value={editName}
              onblur={finishRename}
              onkeydown={(e) => keys(e, finishRename, () => (editingId = null))}
            />
          {:else}
            <button
              class="row"
              class:active={isActive({ kind: "folder", id: f.id })}
              onclick={() => select({ kind: "folder", id: f.id })}
              ondblclick={() => startRename(f.id, f.name)}
            >
              <span class="ico">📁</span><span class="label">{f.name}</span><span class="count">{f.recipe_count}</span>
            </button>
            <button
              class="more"
              title="Folder options"
              onclick={(e) => { e.stopPropagation(); menuFor = menuFor === f.id ? null : f.id; }}
            >⋯</button>
          {/if}
          {#if menuFor === f.id}
            <div class="menu" role="menu">
              <button role="menuitem" onclick={() => startRename(f.id, f.name)}>Rename</button>
              <button role="menuitem" class="danger" onclick={() => remove(f.id, f.name)}>Delete folder</button>
            </div>
          {/if}
        </div>
        {#if !folded[key]}
          {@render items(lib.recipes.filter((r) => r.folder_id === f.id))}
        {/if}
      </div>
    {/each}

    {#if adding}
      <div class="new-folder">
        <input
          bind:value={newName}
          placeholder="Folder name"
          onblur={finishAdd}
          onkeydown={(e) => keys(e, finishAdd, () => (adding = false))}
        />
      </div>
    {:else if lib.folders.length === 0}
      <button class="hint" onclick={startAdd}>Create a folder, then drag recipes onto it</button>
    {/if}

    {#if lib.tags.length > 0}
      <div class="section"><span>Tags</span></div>
      {#each lib.tags as t (t.id)}
        <div class="row-wrap tag-row">
          {#if editingTag === t.id}
            <input
              class="rename rename-tag"
              bind:value={tagName}
              onblur={finishTagRename}
              onkeydown={(e) => keys(e, finishTagRename, () => (editingTag = null))}
            />
          {:else}
            <button
              class="row"
              class:active={isActive({ kind: "tag", name: t.name })}
              onclick={() => select({ kind: "tag", name: t.name })}
              ondblclick={() => startTagRename(t.id, t.name)}
            >
              <span class="ico">#</span><span class="label">{t.name}</span><span class="count">{t.recipe_count}</span>
            </button>
            <button class="more" title="Tag options" onclick={(e) => { e.stopPropagation(); tagMenu = tagMenu === t.id ? null : t.id; }}>⋯</button>
          {/if}
          {#if tagMenu === t.id}
            <div class="menu" role="menu">
              <button role="menuitem" onclick={() => startTagRename(t.id, t.name)}>Rename</button>
              <button role="menuitem" class="danger" onclick={() => removeTag(t.id, t.name)}>Delete tag</button>
            </div>
          {/if}
        </div>
      {/each}
    {/if}

    {#if lib.folders.length === 0 && lib.recipes.length > 0}
      <div class="section"><span>Recipes</span></div>
      {@render items(lib.recipes)}
    {/if}
  </div>

  <button class="row settings" class:active={view === "settings"} onclick={() => onnavigate("settings")}>
    <span class="ico">⚙</span><span class="label">Settings</span>
  </button>
</nav>

{#snippet items(list: typeof lib.recipes)}
  <ul>
    {#each list as r (r.id)}
      <li>
        <button
          class="item"
          class:active={openId === r.id && view === "library"}
          draggable="true"
          ondragstart={(e) => dragStart(e, r.id)}
          onclick={() => { onnavigate("library"); onopen(r.id); }}
          title={r.title}
        >{r.title}</button>
      </li>
    {:else}
      <li class="empty">Empty</li>
    {/each}
  </ul>
{/snippet}

<style>
  nav {
    display: flex;
    flex-direction: column;
    min-height: 0;
    padding: 0 10px 14px;
    background: var(--sidebar);
    backdrop-filter: blur(20px);
    border-right: 1px solid var(--border);
  }
  .drag { height: 16px; flex-shrink: 0; }
  :global(html.mac) .drag { height: 36px; }
  .brand { display: flex; align-items: center; gap: 10px; padding: 4px 10px 16px; }
  .logo { font-size: 22px; }
  .name { font-family: var(--font-display); font-size: 17px; font-weight: 600; }

  .scroll { flex: 1; overflow-y: auto; overflow-x: hidden; margin: 0 -4px; padding: 0 4px; }

  button { border: 0; background: transparent; font: inherit; color: inherit; }
  .row {
    display: flex; align-items: center; gap: 8px; width: 100%; padding: 7px 10px; border-radius: var(--radius-sm);
    text-align: left; color: var(--text-2); transition: background 0.12s, color 0.12s; min-width: 0;
  }
  .row:hover { background: var(--surface-2); color: var(--text); }
  .row.active { background: var(--accent-soft); color: var(--accent); font-weight: 600; }
  .ico { width: 18px; text-align: center; flex-shrink: 0; font-size: 13px; }
  .label { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .count { font-size: 12px; opacity: 0.7; font-variant-numeric: tabular-nums; }

  .row-wrap { position: relative; display: flex; align-items: center; border-radius: var(--radius-sm); }
  .row-wrap .row { padding-left: 4px; }
  .row-wrap.drop { outline: 2px dashed var(--accent); outline-offset: -2px; background: var(--accent-soft); }
  .chev {
    width: 18px; flex-shrink: 0; color: var(--text-2); font-size: 14px; padding: 0; line-height: 1;
    transition: transform 0.15s; transform: rotate(0deg);
  }
  .chev.open { transform: rotate(90deg); }
  .more { opacity: 0; color: var(--text-2); padding: 2px 6px; border-radius: 6px; flex-shrink: 0; }
  .row-wrap:hover .more { opacity: 1; }
  .more:hover { background: var(--surface-2); color: var(--text); }
  .menu {
    position: absolute; right: 4px; top: 100%; z-index: 20; min-width: 150px; padding: 4px;
    background: var(--surface); border: 1px solid var(--border); border-radius: var(--radius-sm); box-shadow: var(--shadow);
  }
  .menu button { display: block; width: 100%; text-align: left; padding: 7px 10px; border-radius: 6px; }
  .menu button:hover { background: var(--surface-2); }
  .menu .danger { color: var(--accent); }
  .rename, .new-folder input {
    flex: 1; min-width: 0; margin: 2px 0; padding: 6px 8px; border-radius: 7px; border: 1px solid var(--accent);
    background: var(--surface); outline: none; user-select: text;
  }
  .new-folder { padding: 2px 4px 2px 22px; display: flex; }

  .section {
    display: flex; align-items: center; justify-content: space-between; margin: 14px 0 4px; padding: 0 10px;
    font-size: 11px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.06em; color: var(--text-2);
  }
  .plus { font-size: 16px; line-height: 1; padding: 0 4px; border-radius: 5px; color: var(--text-2); }
  .plus:hover { background: var(--surface-2); color: var(--text); }
  .hint { display: block; width: 100%; text-align: left; padding: 6px 10px; font-size: 12.5px; color: var(--text-2); }
  .hint:hover { color: var(--accent); }

  ul { list-style: none; margin: 0 0 4px; padding: 0 0 0 22px; }
  .item {
    display: block; width: 100%; text-align: left; padding: 4px 10px; border-radius: 6px; font-size: 13px;
    color: var(--text-2); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; cursor: pointer;
  }
  .item:hover { background: var(--surface-2); color: var(--text); }
  .item.active { color: var(--accent); font-weight: 600; }
  .empty { padding: 3px 10px; font-size: 12px; color: var(--text-2); opacity: 0.6; }
  .settings { margin-top: 8px; flex-shrink: 0; }
  .tag-row { padding-left: 18px; }
  .tag-row .ico { opacity: 0.7; }
</style>
