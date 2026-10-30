<script lang="ts">
  import { onMount } from "svelte";
  import { listRecipes, type RecipeSummary } from "./api";

  let recipes: RecipeSummary[] = $state([]);
  let url = $state("");
  let notice = $state("");

  onMount(async () => {
    recipes = await listRecipes();
  });

  function addRecipe(e: SubmitEvent) {
    e.preventDefault();
    if (!url.trim()) return;
    // URL import arrives in Phase 2.
    notice = "Importing from a link is coming in the next update.";
  }
</script>

<header>
  <h1>Library</h1>
  <form class="add" onsubmit={addRecipe}>
    <input
      type="url"
      placeholder="Paste a recipe link and press Enter"
      bind:value={url}
      oninput={() => (notice = "")}
    />
    <button type="submit">Add recipe</button>
  </form>
  {#if notice}<p class="notice">{notice}</p>{/if}
</header>

{#if recipes.length === 0}
  <section class="empty">
    <div class="plate">🍽️</div>
    <h2>No recipes yet</h2>
    <p>Paste a link from your favourite recipe site above and it will land here as an editable recipe card.</p>
  </section>
{:else}
  <section class="grid">
    {#each recipes as r (r.id)}
      <article class="card">
        <div class="photo"></div>
        <div class="body">
          <h3>{r.title}</h3>
          {#if r.total_min}<span class="chip">{r.total_min} min</span>{/if}
        </div>
      </article>
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
    border: 0;
    border-radius: var(--radius);
    padding: 0 20px;
    background: var(--accent);
    color: #fff;
    font-weight: 600;
  }
  .notice { color: var(--text-2); margin: 10px 4px 0; }

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
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
    box-shadow: var(--shadow);
  }
  .photo { aspect-ratio: 4 / 3; background: var(--surface-2); }
  .body { padding: 12px 14px 14px; }
  .body h3 { margin: 0 0 8px; font-size: 15px; }
  .chip {
    font-size: 12px;
    padding: 3px 8px;
    border-radius: 99px;
    background: var(--accent-soft);
    color: var(--accent);
  }
</style>
