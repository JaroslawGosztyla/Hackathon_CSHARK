<script lang="ts">
  import { strings } from "$lib/strings";

  const ROTATE_MS = 45_000;
  const suggestions = strings.breakView.suggestions;

  let index = $state(Math.floor(Math.random() * suggestions.length));

  function another() {
    index = (index + 1) % suggestions.length;
  }

  $effect(() => {
    const id = setInterval(another, ROTATE_MS);
    return () => clearInterval(id);
  });
</script>

<section class="card break-card" aria-labelledby="break-title">
  <div class="illustration" aria-hidden="true">
    <svg viewBox="0 0 64 64" width="64" height="64">
      <path d="M14 26h30v14a12 12 0 0 1-12 12h-6a12 12 0 0 1-12-12z" style="fill: var(--break)" opacity="0.9" />
      <path d="M44 30h4a6 6 0 0 1 0 12h-4" fill="none" style="stroke: var(--break)" stroke-width="3" />
      <path
        class="steam"
        d="M24 20c-2-3 2-5 0-8M32 20c-2-3 2-5 0-8"
        fill="none"
        style="stroke: var(--text-2)"
        stroke-width="2"
        stroke-linecap="round"
      />
    </svg>
  </div>
  <div class="content">
    <h2 id="break-title">{strings.breakView.title}</h2>
    <p class="secondary">{strings.breakView.lead}</p>
    <div class="suggestion">
      <p class="label">{strings.breakView.suggestionLabel}</p>
      <p class="text">{suggestions[index]}</p>
    </div>
    <button type="button" class="btn btn-sm" onclick={another}>{strings.breakView.another}</button>
  </div>
</section>

<style>
  .break-card {
    display: flex;
    gap: 20px;
    align-items: flex-start;
    border-color: rgba(143, 207, 176, 0.3);
    background: linear-gradient(135deg, rgba(143, 207, 176, 0.1), var(--surface) 60%);
  }

  .illustration {
    flex: none;
    display: grid;
    place-items: center;
    width: 84px;
    height: 84px;
    border-radius: 24px;
    background: var(--break-soft);
  }

  .steam {
    animation: steam 3s ease-in-out infinite;
  }

  @keyframes steam {
    0%,
    100% {
      opacity: 0.3;
      transform: translateY(1px);
    }
    50% {
      opacity: 0.9;
      transform: translateY(-2px);
    }
  }

  .content {
    display: flex;
    flex-direction: column;
    gap: 10px;
    align-items: flex-start;
    min-width: 0;
  }

  .suggestion {
    padding: 12px 16px;
    border-radius: var(--radius-sm);
    background: var(--surface-inset);
    border-left: 3px solid var(--break);
  }

  .label {
    font-size: 0.8rem;
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--break);
    margin-bottom: 4px;
  }

  .text {
    font-size: 1.05rem;
  }

  @media (max-width: 560px) {
    .break-card {
      flex-direction: column;
    }
  }
</style>
