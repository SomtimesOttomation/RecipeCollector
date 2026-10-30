<script lang="ts">
  let { view = $bindable() }: { view: "library" | "settings" } = $props();
</script>

<nav>
  <div class="drag" data-tauri-drag-region></div>
  <div class="brand">
    <span class="logo">🍲</span>
    <span class="name">Lolo's Receitas</span>
  </div>

  <button class:active={view === "library"} onclick={() => (view = "library")}>
    <span class="ico">▦</span> Library
  </button>

  <div class="spacer"></div>

  <button class:active={view === "settings"} onclick={() => (view = "settings")}>
    <span class="ico">⚙</span> Settings
  </button>
</nav>

<style>
  nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 0 12px 16px;
    background: var(--sidebar);
    backdrop-filter: blur(20px);
    border-right: 1px solid var(--border);
  }
  .drag { height: 16px; flex-shrink: 0; }
  :global(html.mac) .drag { height: 36px; }
  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 4px 10px 18px;
  }
  .logo { font-size: 22px; }
  .name {
    font-family: var(--font-display);
    font-size: 17px;
    font-weight: 600;
  }
  button {
    display: flex;
    align-items: center;
    gap: 10px;
    border: 0;
    background: transparent;
    padding: 8px 10px;
    border-radius: var(--radius-sm);
    text-align: left;
    color: var(--text-2);
    transition: background 0.15s, color 0.15s;
  }
  button:hover { background: var(--surface-2); color: var(--text); }
  button.active { background: var(--accent-soft); color: var(--accent); font-weight: 600; }
  .ico { width: 18px; text-align: center; }
  .spacer { flex: 1; }
</style>
