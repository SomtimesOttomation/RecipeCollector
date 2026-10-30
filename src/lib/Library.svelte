<script lang="ts">
  import { onMount } from "svelte";
  import { listRecipes, importRecipe, photoUrl, type RecipeSummary } from "./api";

  let { onopen }: { onopen: (id: number) => void } = $props();

  let recipes = $state<RecipeSummary[]>([]);
  let url = $state("");
  let busy = $state(false);
  let error = $state("");
  let dragging = $state(false);

  onMount(async () => {
    recipes = await listRecipes();
  });

  async function runImport(link: string) {
    const target = link.trim();
    if (!target || busy) return;
    busy = true;
    error = "";
    try {
      const id = await importRecipe(target);
      url = "";
      onopen(id);
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  function submit(e: SubmitEvent) {
    e.preventDefault();
    runImport(url);
  }

  function drop(e: DragEvent) {
    e.preventDefault();
    dragging = false;
    const link = e.dataTransfer?.getData("text/uri-list") || e.dataTransfer?.getData("text/plain") || "";
    const first = link.split(/\s+/).find((l) => l.startsWith("http"));
    if (first) {
      url = first;
      runImport(first);
    }
  }

  function time(min: number | null) {
    if (!min) return "";
    return min >= 60 ? `${Math.floor(min / 60)} h ${min % 60 ? (min % 60) + " min" : ""}`.trim() : `${min} min`;
  }
</script>

<svelte:window
  ondragover={(e) => { e.preventDefault(); dragging = true; }}
  ondragleave={(e) => { if (!e.relatedTarget) dragging = false; }}
  ondrop={drop}
/>

<header>
  <h1>Library</h1>
  <form class="add" class:busy onsubmit={submit}>
    <input
      type="text"
      placeholder="Paste a recipe link and press Enter"
      bind:value={url}
      disabled={busy}
      oninput={() => (error = "")}
    />
    <button type="submit" disabled={busy || !url.trim()}>
      {#if busy}<span class="spinner"></span> Importing…{:else}Add recipe{/if}
    </button>
  </form>
  {#if error}
    <p class="error">{error}</p>
  {:else}
    <p class="hint">Works with most recipe sites and Pinterest pins. You can also drag a link onto the window.</p>
  {/if}
</header>

{#if dragging}
  <div class="dropzone">Drop the link to add the recipe</div>
{/if}

{#if recipes.length === 0}
  <section class="empty">
    <div class="plate">🍽️</div>
    <h2>No recipes yet</h2>
    <p>Paste a link from your favourite recipe site above and it will land here as an editable recipe card.</p>
  </section>
{:else}
  <section class="grid">
    {#each recipes as r (r.id)}
      <button class="card" onclick={() => onopen(r.id)}>
        <div class="photo">
          {#if r.hero_image}
            <img src={photoUrl(r.hero_image)} alt="" loading="lazy" />
          {:else}
            <span>🍲</span>
          {/if}
        </div>
        <div class="body">
          <h3>{r.title}</h3>
          <div class="meta">
            {#if r.total_min}<span class="chip">{time(r.total_min)}</span>{/if}
            {#if r.site_name}<span class="site">{r.site_name}</span>{/if}
          </div>
        </div>
      </button>
    {/each}
  </section>
{/if}

<style>
  h1 {
    font-family: var(--font-display);
    font-size: 30px;
    margin: 0 0 18px;
  }
  .add {
    display: flex;
    gap: 10px;
    max-width: 720px;
  }
  .add input {
    flex: 1;
    padding: 12px 16px;
    border-radius: var(--radius);
    border: 1px solid var(--border);
    background: var(--surface);
    box-shadow: var(--shadow);
    outline: none;
    user-select: text;
  }
  .add input:focus { border-color: var(--accent); }
  .add button {
    display: flex;
    align-items: center;
    gap: 8px;
    border: 0;
    border-radius: var(--radius);
    padding: 0 20px;
    background: var(--accent);
    color: #fff;
    font-weight: 600;
    white-space: nowrap;
  }
  .add button:disabled { opacity: 0.6; cursor: default; }
  .hint { color: var(--text-2); margin: 10px 4px 0; font-size: 13px; }
  .error { color: var(--accent); margin: 10px 4px 0; }

  .spinner {
    width: 14px;
    height: 14px;
    border: 2px solid rgba(255, 255, 255, 0.4);
    border-top-color: #fff;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin { to { transform: rotate(360deg); } }

  .dropzone {
    position: fixed;
    inset: 16px 16px 16px 236px;
    border: 2px dashed var(--accent);
    border-radius: var(--radius);
    background: var(--accent-soft);
    display: grid;
    place-items: center;
    font-size: 18px;
    font-weight: 600;
    color: var(--accent);
    pointer-events: none;
    z-index: 10;
  }

  .empty {
    margin: 80px auto 0;
    max-width: 380px;
    text-align: center;
    color: var(--text-2);
  }
  .plate { font-size: 56px; }
  .empty h2 { color: var(--text); font-family: var(--font-display); margin: 12px 0 6px; }
  .empty p { line-height: 1.5; margin: 0; }

  .grid {
    margin-top: 28px;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 18px;
  }
  .card {
    padding: 0;
    text-align: left;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
    box-shadow: var(--shadow);
    transition: transform 0.15s, box-shadow 0.15s;
  }
  .card:hover { transform: translateY(-2px); }
  .photo {
    aspect-ratio: 4 / 3;
    background: var(--surface-2);
    display: grid;
    place-items: center;
    font-size: 40px;
    overflow: hidden;
  }
  .photo img { width: 100%; height: 100%; object-fit: cover; }
  .body { padding: 12px 14px 14px; }
  .body h3 { margin: 0 0 8px; font-size: 15px; line-height: 1.3; }
  .meta { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; }
  .chip {
    font-size: 12px;
    padding: 3px 8px;
    border-radius: 99px;
    background: var(--accent-soft);
    color: var(--accent);
  }
  .site { font-size: 12px; color: var(--text-2); }
</style>
