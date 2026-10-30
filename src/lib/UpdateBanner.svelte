<script lang="ts">
  import { updates, runInstall } from "./updates.svelte";
  import { openLink, isMacPlatform } from "./api";
</script>

{#if updates.status?.state === "available" && !updates.dismissed}
  {@const s = updates.status}
  <div class="banner">
    <span class="text">
      <strong>Recipe Collector {s.latest}</strong> is available (you have {s.current}).
      {#if updates.error}<span class="err">{updates.error}</span>{/if}
    </span>
    {#if s.installer_url}
      <button class="go" disabled={updates.installing} onclick={runInstall}>
        {updates.installing ? "Downloading…" : isMacPlatform ? "Download update" : "Install and restart"}
      </button>
    {/if}
    <button class="link" onclick={() => openLink(s.page_url)}>What's new</button>
    <button class="x" title="Later" onclick={() => (updates.dismissed = true)}>✕</button>
  </div>
{/if}

<style>
  .banner {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
    margin-bottom: 18px;
    padding: 10px 12px 10px 16px;
    border-radius: var(--radius-sm);
    background: var(--accent-soft);
    border: 1px solid var(--accent);
  }
  .text { flex: 1; min-width: 220px; }
  .err { display: block; color: var(--accent); margin-top: 4px; }
  .go {
    border: 0;
    border-radius: var(--radius-sm);
    padding: 7px 14px;
    background: var(--accent);
    color: #fff;
    font-weight: 600;
  }
  .go:disabled { opacity: 0.6; cursor: default; }
  .link { border: 0; background: none; color: var(--accent); font-weight: 600; }
  .x { border: 0; background: none; color: var(--text-2); }
</style>
