<script lang="ts">
	interface Props {
		open: boolean;
		title: string;
		message: string;
		confirmLabel?: string;
		destructive?: boolean;
		onConfirm: () => void;
		onCancel: () => void;
	}

	let {
		open,
		title,
		message,
		confirmLabel = 'Confirm',
		destructive = false,
		onConfirm,
		onCancel,
	}: Props = $props();
</script>

{#if open}
	<!-- Backdrop -->
	<div
		class="fixed inset-0 bg-black/50 z-40 flex items-center justify-center p-4"
		onclick={onCancel}
		role="presentation"
	>
		<!-- Dialog -->
		<div
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
			role="dialog"
			tabindex="-1"
			aria-modal="true"
			aria-labelledby="confirm-title"
			style="background:var(--c-surface); border:1px solid var(--c-border); border-radius:16px; padding:24px; max-width:420px; width:100%; box-shadow:0 24px 64px rgba(0,0,0,.5);"
		>
			<h2 id="confirm-title" style="font-size:17px; font-weight:600; color:var(--c-text); margin-bottom:8px;">{title}</h2>
			<p style="font-size:13px; color:var(--c-muted); margin-bottom:24px; line-height:1.55;">{message}</p>

			<div class="flex gap-3 justify-end">
				<button
					type="button"
					onclick={onCancel}
					class="px-4 py-2 text-sm font-medium rounded-lg transition"
					style="border:1px solid var(--c-border); color:var(--c-muted); background:transparent;"
					onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.borderColor='var(--c-border-hi)'; (e.currentTarget as HTMLElement).style.color='var(--c-text)'; }}
					onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.borderColor='var(--c-border)'; (e.currentTarget as HTMLElement).style.color='var(--c-muted)'; }}
				>Abbrechen</button>
				<button
					type="button"
					onclick={onConfirm}
					class="px-4 py-2 text-sm font-semibold rounded-lg transition"
					style="{destructive ? 'background:#dc2626; color:#fff;' : 'background:#7c7cff; color:#05050f;'}"
					onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.opacity='.85'; }}
					onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.opacity='1'; }}
				>{confirmLabel}</button>
			</div>
		</div>
	</div>
{/if}
