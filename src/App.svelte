<script lang="ts">
  import { onMount } from "svelte";
  import Sidebar from "./lib/Sidebar.svelte";
  import Library from "./lib/Library.svelte";
  import RecipePage from "./lib/RecipePage.svelte";
  import Settings from "./lib/Settings.svelte";
  import UpdateBanner from "./lib/UpdateBanner.svelte";
  import { runCheck } from "./lib/updates.svelte";
  import { autoCheckEnabled } from "./lib/api";
  import { refresh } from "./lib/store.svelte";
  import { applyTheme, loadTheme, isMac, type Theme } from "./lib/theme";

  type View = "library" | "settings";
  let view: View = $state("library");
  let openId: number | null = $state(null);
  let theme: Theme = $state(loadTheme());

  $effect(() => applyTheme(theme));

  onMount(() => {
    document.documentElement.classList.add(isMac ? "mac" : "win");
    // One quiet check a few seconds after start-up, unless turned off in Settings.
    if (autoCheckEnabled()) setTimeout(runCheck, 4000);
  });

  function go(v: View) {
    view = v;
    openId = null;
  }

  function closeRecipe() {
    openId = null;
    refresh(); // titles, photos or folders may have changed
  }
</script>

<div class="shell">
  <Sidebar {view} {openId} onnavigate={go} onopen={(id) => (openId = id)} />
  <main>
    <UpdateBanner />
    {#if view === "settings"}
      <Settings bind:theme />
    {:else if openId !== null}
      {#key openId}
        <RecipePage id={openId} onclose={closeRecipe} />
      {/key}
    {:else}
      <Library onopen={(id) => (openId = id)} />
    {/if}
  </main>
</div>

<style>
  .shell {
    display: grid;
    grid-template-columns: 220px 1fr;
    height: 100%;
  }
  main {
    overflow: auto;
    padding: 28px 36px 40px;
  }
  :global(html.mac) main { padding-top: 40px; }
</style>
