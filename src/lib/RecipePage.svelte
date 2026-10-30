<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import {
    getRecipe, updateRecipe, deleteRecipe, addImageFromFile, setHero, removeImage,
    pickPhoto, confirmDelete, openLink, photoUrl, inTauri, youtubeId, printPage, type Recipe, type Section,
  } from "./api";
  import { lib, moveTo } from "./store.svelte";
  import { convertText, detectSystem, systemLabel, loadUnitMode, saveUnitMode, type UnitMode } from "./units";

  let { id, onclose }: { id: number; onclose: () => void } = $props();

  let recipe = $state<Recipe | null>(null);
  let loadError = $state("");
  let status = $state<"" | "saving" | "saved" | "error">("");
  let saveError = $state("");
  let lastSaved = "";
  let timer: ReturnType<typeof setTimeout> | undefined;

  onMount(async () => {
    try {
      recipe = await getRecipe(id);
      lastSaved = snapshot(recipe);
    } catch (e) {
      loadError = String(e);
    }
  });

  onDestroy(() => {
    // Save anything pending when leaving the page.
    if (timer) { clearTimeout(timer); flush(); }
  });

  function snapshot(r: Recipe) {
    const { images, ...rest } = r;
    return JSON.stringify(rest);
  }

  function toNum(v: unknown): number | null {
    const n = typeof v === "number" ? v : parseInt(String(v ?? ""), 10);
    return Number.isFinite(n) && n > 0 ? Math.round(n) : null;
  }

  function cleaned(r: Recipe): Recipe {
    const blank = (s: string | null) => (s && s.trim() ? s : null);
    const sections = (list: Section[]) =>
      list
        .map((s) => ({ title: blank(s.title), items: s.items.filter((i) => i.trim()) }))
        .filter((s) => s.items.length || s.title);
    return {
      ...r,
      title: r.title.trim() || "Untitled recipe",
      description: blank(r.description),
      video_url: blank(r.video_url),
      servings: blank(r.servings),
      notes: blank(r.notes),
      prep_min: toNum(r.prep_min),
      cook_min: toNum(r.cook_min),
      total_min: toNum(r.total_min),
      ingredients: sections(r.ingredients),
      steps: sections(r.steps),
    };
  }

  async function flush() {
    if (!recipe) return;
    const snap = snapshot(recipe);
    if (snap === lastSaved) return;
    status = "saving";
    try {
      await updateRecipe(cleaned($state.snapshot(recipe) as Recipe));
      lastSaved = snap;
      status = "saved";
    } catch (e) {
      status = "error";
      saveError = String(e);
    }
  }

  // Autosave shortly after each edit.
  $effect(() => {
    if (!recipe) return;
    const snap = snapshot(recipe);
    if (snap === lastSaved) return;
    clearTimeout(timer);
    status = "saving";
    timer = setTimeout(() => { timer = undefined; flush(); }, 700);
  });

  // ---- list editing ----
  function addItem(list: Section[], si: number) {
    list[si].items.push("");
    focusLast(`[data-sec="${si}"]`);
  }
  function removeItem(list: Section[], si: number, ii: number) {
    list[si].items.splice(ii, 1);
  }
  function moveItem(list: Section[], si: number, ii: number, dir: -1 | 1) {
    const items = list[si].items;
    const j = ii + dir;
    if (j < 0 || j >= items.length) return;
    [items[ii], items[j]] = [items[j], items[ii]];
  }
  function addSection(list: Section[]) {
    list.push({ title: "", items: [""] });
  }
  function focusLast(scope: string) {
    queueMicrotask(() => {
      const fields = document.querySelectorAll<HTMLElement>(`${scope} .item-field`);
      fields[fields.length - 1]?.focus();
    });
  }
  function onItemKey(e: KeyboardEvent, list: Section[], si: number, ii: number) {
    if (e.key === "Enter" && !e.shiftKey && (e.target as HTMLElement).tagName === "INPUT") {
      e.preventDefault();
      list[si].items.splice(ii + 1, 0, "");
      queueMicrotask(() => {
        const fields = document.querySelectorAll<HTMLElement>(`[data-sec="${si}"] .item-field`);
        fields[ii + 1]?.focus();
      });
    }
  }
  function stepNumber(si: number, ii: number) {
    if (!recipe) return ii + 1;
    let n = 0;
    for (let s = 0; s < si; s++) n += recipe.steps[s].items.length;
    return n + ii + 1;
  }

  // ---- photos ----
  async function addPhoto(makeHero: boolean) {
    if (!recipe) return;
    const path = await pickPhoto();
    if (!path) return;
    try {
      await addImageFromFile(recipe.id, path, makeHero);
      recipe.images = (await getRecipe(recipe.id)).images;
    } catch (e) {
      saveError = String(e);
      status = "error";
    }
  }
  async function makeCover(imageId: number) {
    if (!recipe) return;
    await setHero(imageId);
    recipe.images = (await getRecipe(recipe.id)).images;
  }
  async function dropPhoto(imageId: number) {
    if (!recipe) return;
    await removeImage(imageId);
    recipe.images = (await getRecipe(recipe.id)).images;
  }

  async function remove() {
    if (!recipe) return;
    if (await confirmDelete(recipe.title)) {
      clearTimeout(timer);
      timer = undefined;
      await deleteRecipe(recipe.id);
      onclose();
    }
  }

  function autosize(node: HTMLTextAreaElement) {
    const fit = () => {
      node.style.height = "auto";
      node.style.height = node.scrollHeight + "px";
    };
    queueMicrotask(fit);
    node.addEventListener("input", fit);
    return { destroy: () => node.removeEventListener("input", fit) };
  }

  let unitMode = $state<UnitMode>(loadUnitMode());
  function setMode(m: UnitMode) {
    unitMode = m;
    saveUnitMode(m);
  }
  let written = $derived(
    recipe ? detectSystem([...(recipe as Recipe).ingredients.flatMap((s) => s.items), ...(recipe as Recipe).steps.flatMap((s) => s.items)]) : "none",
  );

  let newTag = $state("");
  function addTag() {
    const t = newTag.trim().replace(/^#/, "");
    if (!recipe || !t) return;
    if (!recipe.tags.some((x) => x.toLowerCase() === t.toLowerCase())) recipe.tags.push(t);
    newTag = "";
  }
  function tagKey(e: KeyboardEvent) {
    if (e.key === "Enter" || e.key === ",") {
      e.preventDefault();
      addTag();
    } else if (e.key === "Backspace" && !newTag && recipe?.tags.length) {
      recipe.tags.pop();
    }
  }

  let folderId = $derived(lib.recipes.find((r) => r.id === id)?.folder_id ?? null);
  async function changeFolder(e: Event) {
    const v = (e.target as HTMLSelectElement).value;
    await moveTo(id, v === "" ? null : parseInt(v, 10));
  }

  let hero = $derived.by(() => {
    const r = recipe as Recipe | null;
    return r ? (r.images.find((i) => i.is_hero) ?? r.images[0] ?? null) : null;
  });
</script>

{#if loadError}
  <p class="error">Couldn't open this recipe: {loadError}</p>
  <button class="back" onclick={onclose}>← Library</button>
{:else if recipe}
  <div class="topbar">
    <button class="back" onclick={onclose}>← Library</button>
    <span class="status">
      {#if status === "saving"}Saving…{:else if status === "saved"}Saved{:else if status === "error"}<span class="error">{saveError}</span>{/if}
    </span>
    <button class="ghost" onclick={async () => { await flush(); printPage(); }}>Print</button>
    <button class="ghost danger" onclick={remove}>Delete</button>
  </div>

  <section class="hero" class:empty={!hero}>
    {#if hero}
      <img src={photoUrl(hero.path)} alt="" />
    {/if}
    <div class="hero-actions">
      {#if inTauri}
        <button class="pill" onclick={() => addPhoto(true)}>{hero ? "Replace cover" : "Add a cover photo"}</button>
      {/if}
    </div>
  </section>

  <header class="head">
    <input class="title" bind:value={recipe.title} placeholder="Recipe name" />
    <textarea class="desc" use:autosize bind:value={recipe.description} placeholder="Add a short description" rows="1"></textarea>
    <div class="facts">
      <label>Serves <input bind:value={recipe.servings} placeholder="—" /></label>
      <label>Prep <input type="number" min="0" bind:value={recipe.prep_min} placeholder="—" /> min</label>
      <label>Cook <input type="number" min="0" bind:value={recipe.cook_min} placeholder="—" /> min</label>
      <label>Total <input type="number" min="0" bind:value={recipe.total_min} placeholder="—" /> min</label>
      {#if lib.folders.length > 0}
        <label class="folder-pick">📁
          <select value={folderId === null ? "" : String(folderId)} onchange={changeFolder}>
            <option value="">Not in a folder</option>
            {#each lib.folders as f (f.id)}<option value={String(f.id)}>{f.name}</option>{/each}
          </select>
        </label>
      {/if}
      {#if recipe.source_url}
        <button class="link" onclick={() => recipe?.source_url && openLink(recipe.source_url)}>
          {recipe.site_name ?? "Source"} ↗
        </button>
      {/if}
    </div>
    <div class="tags">
      {#each recipe.tags as t, i (t)}
        <span class="tag">#{t}<button title="Remove tag" onclick={() => recipe?.tags.splice(i, 1)}>✕</button></span>
      {/each}
      <input
        class="tag-input"
        list="all-tags"
        bind:value={newTag}
        onkeydown={tagKey}
        onblur={addTag}
        placeholder={recipe.tags.length ? "Add tag" : "Add tags, e.g. Dessert, Quick"}
      />
      <datalist id="all-tags">
        {#each lib.tags.filter((t) => !recipe?.tags.some((x) => x.toLowerCase() === t.name.toLowerCase())) as t (t.id)}
          <option value={t.name}></option>
        {/each}
      </datalist>
    </div>
  </header>

  <div class="units">
    <span class="units-label">Units</span>
    <div class="seg">
      {#each [["auto", "Auto"], ["metric", "Metric"], ["us", "US"]] as [m, label]}
        <button class:on={unitMode === m} onclick={() => setMode(m as UnitMode)}>{label}</button>
      {/each}
    </div>
    <span class="units-note">
      {#if unitMode === "auto"}
        As written: {systemLabel(written)}.
      {:else if written === "none"}
        Nothing to convert in this recipe.
      {:else}
        Converted for reading. Switch to Auto to edit.
      {/if}
    </span>
  </div>

  {#if unitMode === "auto"}
  <div class="cards">
    <section class="card">
      <h2>Ingredients</h2>
      {#if recipe.ingredients.length === 0}
        <p class="missing">No ingredients were found on the page. Add them here.</p>
      {/if}
      {#each recipe.ingredients as sec, si}
        <div class="section" data-sec={si}>
          {#if sec.title !== null || recipe.ingredients.length > 1}
            <input class="sec-title" bind:value={sec.title} placeholder="Section name (optional)" />
          {/if}
          <ul class="ingredients">
            {#each sec.items as _, ii}
              <li>
                <span class="dot"></span>
                <input class="item-field" bind:value={sec.items[ii]} placeholder="e.g. 2 cups flour"
                  onkeydown={(e) => recipe && onItemKey(e, recipe.ingredients, si, ii)} />
                <span class="row-tools">
                  <button title="Move up" onclick={() => recipe && moveItem(recipe.ingredients, si, ii, -1)}>↑</button>
                  <button title="Move down" onclick={() => recipe && moveItem(recipe.ingredients, si, ii, 1)}>↓</button>
                  <button title="Remove" onclick={() => recipe && removeItem(recipe.ingredients, si, ii)}>✕</button>
                </span>
              </li>
            {/each}
          </ul>
          <button class="add" onclick={() => recipe && addItem(recipe.ingredients, si)}>+ Add ingredient</button>
        </div>
      {/each}
      <button class="add subtle" onclick={() => recipe && (recipe.ingredients.length ? addSection(recipe.ingredients) : recipe.ingredients.push({ title: null, items: [""] }))}>
        {recipe.ingredients.length ? "+ Add section" : "+ Add ingredients"}
      </button>
    </section>

    <section class="card">
      <h2>Steps</h2>
      {#if recipe.steps.length === 0}
        <p class="missing">No steps were found on the page. Add them here.</p>
      {/if}
      {#each recipe.steps as sec, si}
        <div class="section" data-sec={`s${si}`}>
          {#if sec.title !== null || recipe.steps.length > 1}
            <input class="sec-title" bind:value={sec.title} placeholder="Section name (optional)" />
          {/if}
          <ol class="steps">
            {#each sec.items as _, ii}
              <li>
                <span class="num">{stepNumber(si, ii)}</span>
                <textarea class="item-field" use:autosize bind:value={sec.items[ii]} rows="1" placeholder="Describe this step"></textarea>
                <span class="row-tools">
                  <button title="Move up" onclick={() => recipe && moveItem(recipe.steps, si, ii, -1)}>↑</button>
                  <button title="Move down" onclick={() => recipe && moveItem(recipe.steps, si, ii, 1)}>↓</button>
                  <button title="Remove" onclick={() => recipe && removeItem(recipe.steps, si, ii)}>✕</button>
                </span>
              </li>
            {/each}
          </ol>
          <button class="add" onclick={() => { if (recipe) { recipe.steps[si].items.push(""); focusLast(`[data-sec="s${si}"]`); } }}>+ Add step</button>
        </div>
      {/each}
      <button class="add subtle" onclick={() => recipe && (recipe.steps.length ? addSection(recipe.steps) : recipe.steps.push({ title: null, items: [""] }))}>
        {recipe.steps.length ? "+ Add section" : "+ Add steps"}
      </button>
    </section>
  </div>
  {:else}
    <div class="cards converted">
      <section class="card">
        <h2>Ingredients</h2>
        {#each recipe.ingredients as sec}
          {#if sec.title}<div class="ro-title">{sec.title}</div>{/if}
          <ul class="ingredients">
            {#each sec.items.filter((i) => i.trim()) as item}
              {@const conv = convertText(item, unitMode)}
              <li><span class="dot"></span><span class="ro" class:changed={conv !== item} title={conv !== item ? `Original: ${item}` : undefined}>{conv}</span></li>
            {/each}
          </ul>
        {/each}
      </section>
      <section class="card">
        <h2>Steps</h2>
        {#each recipe.steps as sec, si}
          {#if sec.title}<div class="ro-title">{sec.title}</div>{/if}
          <ol class="steps">
            {#each sec.items.filter((i) => i.trim()) as item, ii}
              {@const conv = convertText(item, unitMode)}
              <li><span class="num">{stepNumber(si, ii)}</span><span class="ro" class:changed={conv !== item} title={conv !== item ? `Original: ${item}` : undefined}>{conv}</span></li>
            {/each}
          </ol>
        {/each}
      </section>
    </div>
  {/if}

  <section class="card video">
    <h2>Video</h2>
    {#if youtubeId(recipe.video_url)}
      {@const vid = youtubeId(recipe.video_url)}
      <button class="video-card" onclick={() => recipe?.video_url && openLink(recipe.video_url)} title="Watch on YouTube">
        <img src={`https://i.ytimg.com/vi/${vid}/hqdefault.jpg`} alt="" loading="lazy" />
        <span class="play">▶</span>
      </button>
    {/if}
    <div class="video-row">
      <input bind:value={recipe.video_url} placeholder="Paste a YouTube link" />
      {#if recipe.video_url && !youtubeId(recipe.video_url)}
        <button class="link" onclick={() => recipe?.video_url && openLink(recipe.video_url)}>Open ↗</button>
      {/if}
    </div>
  </section>

  <section class="card notes">
    <h2>Notes</h2>
    <textarea use:autosize bind:value={recipe.notes} rows="2" placeholder="Your tweaks, substitutions, who loved it…"></textarea>
  </section>

  <section class="card photos">
    <h2>Photos</h2>
    <div class="gallery">
      {#each recipe.images as img (img.id)}
        <figure class:cover={img.is_hero}>
          <img src={photoUrl(img.path)} alt="" loading="lazy" />
          <figcaption>
            {#if img.is_hero}
              <span class="badge">Cover</span>
            {:else}
              <button onclick={() => makeCover(img.id)}>Set as cover</button>
            {/if}
            <button onclick={() => dropPhoto(img.id)}>Remove</button>
          </figcaption>
        </figure>
      {/each}
      {#if inTauri}
        <button class="add-photo" onclick={() => addPhoto(false)}>+ Add photo</button>
      {/if}
    </div>
  </section>

  <!-- Clean, read-only copy used only when printing. -->
  <article class="print-view" aria-hidden="true">
    <h1>{recipe.title}</h1>
    {#if recipe.description}<p class="p-desc">{recipe.description}</p>{/if}
    <p class="p-facts">
      {#if recipe.servings}<span>Serves {recipe.servings}</span>{/if}
      {#if recipe.prep_min}<span>Prep {recipe.prep_min} min</span>{/if}
      {#if recipe.cook_min}<span>Cook {recipe.cook_min} min</span>{/if}
      {#if recipe.total_min}<span>Total {recipe.total_min} min</span>{/if}
    </p>
    {#if hero}<img class="p-hero" src={photoUrl(hero.path)} alt="" />{/if}
    <div class="p-cols">
      <section>
        <h2>Ingredients</h2>
        {#each recipe.ingredients as sec}
          {#if sec.title}<h3>{sec.title}</h3>{/if}
          <ul>{#each sec.items.filter((i) => i.trim()) as item}<li>{convertText(item, unitMode)}</li>{/each}</ul>
        {/each}
      </section>
      <section>
        <h2>Steps</h2>
        {#each recipe.steps as sec, si}
          {#if sec.title}<h3>{sec.title}</h3>{/if}
          <ol start={stepNumber(si, 0)}>{#each sec.items.filter((i) => i.trim()) as item}<li>{convertText(item, unitMode)}</li>{/each}</ol>
        {/each}
      </section>
    </div>
    {#if recipe.notes}
      <section><h2>Notes</h2><p class="p-notes">{recipe.notes}</p></section>
    {/if}
    {#if recipe.source_url}<p class="p-source">Source: {recipe.source_url}</p>{/if}
  </article>
{:else}
  <p class="loading">Loading…</p>
{/if}

<style>
  .print-view { display: none; }
  @media print {
    :global(nav), :global(.banner), .topbar, .hero, .head, .units, .cards, .notes, .photos, .video { display: none !important; }
    :global(html), :global(body), :global(main) { background: #fff !important; color: #000 !important; overflow: visible !important; height: auto !important; }
    :global(.shell) { display: block !important; height: auto !important; }
    :global(main) { padding: 0 !important; }
    .print-view { display: block; font-size: 11pt; line-height: 1.45; color: #000; }
    .print-view h1 { font-family: var(--font-display); font-size: 24pt; margin: 0 0 4pt; }
    .print-view h2 { font-family: var(--font-display); font-size: 14pt; margin: 14pt 0 6pt; border-bottom: 1px solid #ccc; padding-bottom: 3pt; }
    .print-view h3 { font-size: 10pt; text-transform: uppercase; letter-spacing: 0.04em; margin: 8pt 0 4pt; color: #444; }
    .p-desc { margin: 0 0 6pt; color: #333; }
    .p-facts { display: flex; gap: 14pt; margin: 0 0 10pt; color: #333; }
    .p-hero { width: 100%; max-height: 3in; object-fit: cover; border-radius: 6pt; margin-bottom: 6pt; }
    .p-cols { display: grid; grid-template-columns: 1fr 1.6fr; gap: 20pt; }
    .print-view ul, .print-view ol { margin: 0; padding-left: 16pt; list-style: revert; }
    .print-view li { padding: 0; display: list-item; margin-bottom: 3pt; break-inside: avoid; }
    .p-notes { white-space: pre-wrap; margin: 0; }
    .p-source { margin-top: 14pt; font-size: 8.5pt; color: #666; word-break: break-all; }
  }
  .topbar { display: flex; align-items: center; gap: 12px; margin-bottom: 16px; }
  .status { flex: 1; color: var(--text-2); font-size: 13px; }
  .back, .ghost {
    border: 0; background: none; padding: 6px 10px; border-radius: var(--radius-sm); color: var(--text-2); font-weight: 600;
  }
  .back:hover, .ghost:hover { background: var(--surface-2); color: var(--text); }
  .danger:hover { color: var(--accent); }
  .error { color: var(--accent); }
  .loading { color: var(--text-2); }

  .hero {
    position: relative;
    border-radius: var(--radius);
    overflow: hidden;
    aspect-ratio: 21 / 8;
    max-height: 380px;
    width: 100%;
    background: var(--surface-2);
    box-shadow: var(--shadow);
  }
  .hero.empty { aspect-ratio: auto; height: 120px; display: grid; place-items: center; }
  .hero img { width: 100%; height: 100%; object-fit: cover; display: block; }
  .hero-actions { position: absolute; right: 14px; bottom: 14px; }
  .hero.empty .hero-actions { position: static; }
  .pill {
    border: 0; border-radius: 99px; padding: 8px 14px; font-weight: 600;
    background: rgba(255, 255, 255, 0.9); color: #2a2420; box-shadow: var(--shadow);
  }

  input, textarea { user-select: text; }
  .head { margin: 22px 0 18px; }
  .title {
    width: 100%; border: 0; background: none; outline: none;
    font-family: var(--font-display); font-size: 32px; font-weight: 700; padding: 0;
  }
  .desc {
    width: 100%; border: 0; background: none; outline: none; resize: none;
    color: var(--text-2); font: inherit; font-size: 15px; line-height: 1.5; margin-top: 6px; padding: 0;
  }
  .facts { display: flex; flex-wrap: wrap; gap: 8px; margin-top: 12px; align-items: center; }
  .facts label {
    display: inline-flex; align-items: center; gap: 6px; font-size: 13px; color: var(--text-2);
    background: var(--surface); border: 1px solid var(--border); border-radius: 99px; padding: 5px 12px;
  }
  .facts input {
    width: 54px; border: 0; background: none; outline: none; font-weight: 600; color: var(--text); text-align: center;
  }
  .facts label:first-child input { width: 110px; text-align: left; }
  .facts input::-webkit-inner-spin-button { display: none; }
  .units { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; margin: 0 0 12px; }
  .units-label { font-size: 12px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em; color: var(--text-2); }
  .seg { display: inline-flex; background: var(--surface-2); border-radius: var(--radius-sm); padding: 3px; }
  .seg button { border: 0; background: transparent; padding: 5px 14px; border-radius: 7px; color: var(--text-2); font-weight: 600; }
  .seg button.on { background: var(--surface); color: var(--text); box-shadow: var(--shadow); }
  .units-note { font-size: 13px; color: var(--text-2); }
  .ro { flex: 1; font-size: 14.5px; line-height: 1.5; padding: 2px 4px; user-select: text; }
  .ro.changed { text-decoration: underline dotted var(--accent); text-underline-offset: 3px; }
  .ro-title { font-weight: 700; font-size: 13px; text-transform: uppercase; letter-spacing: 0.04em; color: var(--text-2); margin: 8px 0 4px; }
  .tags { display: flex; flex-wrap: wrap; gap: 6px; margin-top: 10px; align-items: center; }
  .tag {
    display: inline-flex; align-items: center; gap: 4px; font-size: 12.5px; padding: 3px 6px 3px 10px;
    border-radius: 99px; background: var(--accent-soft); color: var(--accent); font-weight: 600;
  }
  .tag button { border: 0; background: none; color: inherit; opacity: 0.6; font-size: 10px; padding: 2px 3px; }
  .tag button:hover { opacity: 1; }
  .tag-input {
    border: 0; background: none; outline: none; font: inherit; font-size: 13px; min-width: 150px; padding: 4px 2px;
    color: var(--text);
  }
  .folder-pick select {
    border: 0; background: none; outline: none; font: inherit; font-weight: 600; color: var(--text); cursor: pointer;
  }
  .link { border: 0; background: none; color: var(--accent); font-weight: 600; font-size: 13px; padding: 5px 8px; }

  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
    gap: 18px;
    align-items: start;
  }
  .card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 18px 20px 14px;
    box-shadow: var(--shadow);
  }
  .card h2 { font-family: var(--font-display); font-size: 19px; margin: 0 0 12px; }
  .notes, .photos, .video { margin-top: 18px; }
  .video-card {
    position: relative; display: block; padding: 0; border: 0; background: #000;
    width: 100%; max-width: 480px; aspect-ratio: 16 / 9; border-radius: var(--radius-sm); overflow: hidden; margin-bottom: 10px;
  }
  .video-card img { width: 100%; height: 100%; object-fit: cover; display: block; opacity: 0.92; }
  .video-card .play {
    position: absolute; inset: 0; margin: auto; width: 64px; height: 44px; border-radius: 12px;
    background: rgba(220, 30, 30, 0.92); color: #fff; display: grid; place-items: center; font-size: 20px;
  }
  .video-card:hover img { opacity: 1; }
  .video-row { display: flex; gap: 10px; align-items: center; }
  .video-row input {
    flex: 1; max-width: 480px; border: 1px solid var(--border); background: var(--surface-2); border-radius: var(--radius-sm);
    padding: 7px 10px; outline: none; font: inherit; font-size: 13.5px;
  }
  .video-row input:focus { border-color: var(--accent); }
  .missing { color: var(--text-2); font-size: 13px; margin: 0 0 8px; }

  .section + .section { margin-top: 14px; }
  .sec-title {
    width: 100%; border: 0; border-bottom: 1px dashed var(--border); background: none; outline: none;
    font-weight: 700; font-size: 13px; text-transform: uppercase; letter-spacing: 0.04em; color: var(--text-2);
    padding: 4px 0; margin-bottom: 6px;
  }
  ul, ol { list-style: none; margin: 0; padding: 0; }
  li { display: flex; align-items: flex-start; gap: 10px; padding: 4px 0; border-radius: 8px; }
  li:hover .row-tools, li:focus-within .row-tools { opacity: 1; }
  .dot { width: 6px; height: 6px; border-radius: 50%; background: var(--accent); margin-top: 9px; flex-shrink: 0; }
  .num {
    flex-shrink: 0; width: 24px; height: 24px; border-radius: 50%; background: var(--accent-soft); color: var(--accent);
    display: grid; place-items: center; font-size: 12px; font-weight: 700; margin-top: 1px;
  }
  .item-field {
    flex: 1; border: 0; background: none; outline: none; font: inherit; font-size: 14.5px; line-height: 1.5;
    padding: 2px 4px; border-radius: 6px; resize: none; min-width: 0;
  }
  .item-field:focus { background: var(--surface-2); }
  .row-tools { display: flex; gap: 2px; opacity: 0; transition: opacity 0.12s; }
  .row-tools button {
    border: 0; background: none; color: var(--text-2); width: 24px; height: 24px; border-radius: 6px; font-size: 12px;
  }
  .row-tools button:hover { background: var(--surface-2); color: var(--text); }
  .add {
    border: 0; background: none; color: var(--accent); font-weight: 600; font-size: 13px; padding: 6px 0;
  }
  .add.subtle { color: var(--text-2); margin-top: 6px; }

  .notes textarea {
    width: 100%; border: 0; background: none; outline: none; resize: none; font: inherit; font-size: 14.5px; line-height: 1.5;
  }

  .gallery { display: flex; flex-wrap: wrap; gap: 12px; }
  figure { margin: 0; width: 160px; }
  figure img { width: 160px; height: 120px; object-fit: cover; border-radius: var(--radius-sm); display: block; }
  figure.cover img { outline: 2px solid var(--accent); outline-offset: 2px; }
  figcaption { display: flex; gap: 6px; margin-top: 6px; align-items: center; }
  figcaption button {
    border: 0; background: var(--surface-2); border-radius: 6px; padding: 3px 8px; font-size: 12px; color: var(--text-2);
  }
  figcaption button:hover { color: var(--text); }
  .badge { font-size: 12px; font-weight: 700; color: var(--accent); padding: 3px 4px; }
  .add-photo {
    width: 160px; height: 120px; border: 2px dashed var(--border); border-radius: var(--radius-sm);
    background: none; color: var(--text-2); font-weight: 600;
  }
  .add-photo:hover { border-color: var(--accent); color: var(--accent); }
</style>
