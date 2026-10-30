<script lang="ts">
  import { onMount } from "svelte";
  import Sidebar from "./lib/Sidebar.svelte";
  import Library from "./lib/Library.svelte";
  import RecipePage from "./lib/RecipePage.svelte";
  import Settings from "./lib/Settings.svelte";
  import { applyTheme, loadTheme, isMac, type Theme } from "./lib/theme";

  type View = "library" | "settings";
  let view: View = $state("library");
  let openId: number | null = $state(null);
  let theme: Theme = $state(loadTheme());

  $effect(() => applyTheme(theme));

  onMount(() => {
    document.documentElement.classList.add(isMac ? "mac" : "win");
  });

  function go(v: View) {
    view = v;
    openId = null;
  }
</script>

<div class="shell">
  <Sidebar {view} onnavigate={go} />
  <main>
    {#if view === "settings"}
      <Settings bind:theme />
    {:else if openId !== null}
      {#key openId}
        <RecipePage id={openId} onclose={() => (openId = null)} />
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
