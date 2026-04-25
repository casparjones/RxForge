<script lang="ts">
  interface Props {
    size?: number;
    color?: string;
    accent?: string;
    /** 'lockup' = mark + wordmark, 'mark' = bolt only, 'wordmark' = text only */
    variant?: 'lockup' | 'mark' | 'wordmark';
    class?: string;
  }

  let {
    size = 32,
    color = 'currentColor',
    accent = '#7c7cff',
    variant = 'lockup',
    class: cls = '',
  }: Props = $props();

  const markSize = $derived(size * 1.05);
  const textSize = $derived(size * 0.95);
</script>

<div class="inline-flex items-center gap-[0.3em] {cls}" style="line-height:1">
  {#if variant === 'lockup' || variant === 'mark'}
    <!-- Bolt mark — 64×64 design grid -->
    <svg
      width={markSize}
      height={markSize}
      viewBox="0 0 64 64"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      aria-hidden="true"
    >
      <path
        d="M36 6 L14 36 L28 36 L24 58 L48 26 L34 26 L36 6 Z"
        stroke={accent}
        stroke-width="3"
        stroke-linejoin="round"
        stroke-linecap="round"
        fill="none"
      />
    </svg>
  {/if}

  {#if variant === 'lockup' || variant === 'wordmark'}
    <span
      style="
        font-family: 'Space Grotesk', system-ui, sans-serif;
        font-weight: 700;
        font-size: {textSize}px;
        letter-spacing: -0.025em;
        color: {color};
        line-height: 1;
      "
    ><span style="color:{accent}">Rx</span>Forge</span>
  {/if}
</div>
