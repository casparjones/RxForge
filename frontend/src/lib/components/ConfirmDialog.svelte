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
			class="bg-white rounded-2xl shadow-xl max-w-md w-full p-6 z-50"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
			role="dialog"
			tabindex="-1"
			aria-modal="true"
			aria-labelledby="confirm-title"
		>
			<h2 id="confirm-title" class="text-lg font-semibold text-gray-900 mb-2">{title}</h2>
			<p class="text-gray-600 text-sm mb-6">{message}</p>

			<div class="flex gap-3 justify-end">
				<button
					type="button"
					onclick={onCancel}
					class="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-lg hover:bg-gray-50 transition"
				>
					Cancel
				</button>
				<button
					type="button"
					onclick={onConfirm}
					class="px-4 py-2 text-sm font-medium rounded-lg transition {destructive
						? 'bg-red-600 hover:bg-red-700 text-white'
						: 'bg-indigo-600 hover:bg-indigo-700 text-white'}"
				>
					{confirmLabel}
				</button>
			</div>
		</div>
	</div>
{/if}
