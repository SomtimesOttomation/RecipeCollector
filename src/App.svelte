<script lang="ts">
  import { onMount } from "svelte";
  import Sidebar from "./lib/Sidebar.svelte";
  import Library from "./lib/Library.svelte";
  import Settings from "./lib/Settings.svelte";
  import { applyTheme, loadTheme, isMac, type Theme } from "./lib/theme";

  type View = "library" | "settings";
  let view: View = $state("library");
  let theme: Theme = $state(loadTheme());

  $effect(() => applyTheme(theme));

  onMount(() => {
    document.documentElement.classList.add(isMac ? "mac" : "win");
  });
</script>

<div class="shell">
  <Sidebar bind:view />
  <main>
    {#if view === "library"}
      <Library />
    {:else}
      <Settings bind:theme />
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
