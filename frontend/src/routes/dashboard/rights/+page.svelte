<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import { toast } from '$lib/stores/toast';

	type Right = { client_id: string; app_name: string; granted_at: string };

	let rights = $state<Right[]>([]);
	let loading = $state(true);
	let error = $state('');
	let revoking = $state<string | null>(null);

	onMount(loadRights);

	async function loadRights() {
		loading = true; error = '';
		try { rights = await api.rights.list(); }
		catch (e: any) { error = e.message; }
		finally { loading = false; }
	}

	async function revoke(client_id: string, app_name: string) {
		if (!confirm(`Revoke access for "${app_name}"? The app will no longer be able to act on your behalf.`)) return;
		revoking = client_id;
		try {
			await api.rights.revoke(client_id);
			rights = rights.filter(r => r.client_id !== client_id);
			toast.success(`Access revoked for ${app_name}`);
		} catch (e: any) {
			toast.error(e.message);
		} finally {
			revoking = null;
		}
	}

	function formatDate(iso: string) {
		return new Intl.DateTimeFormat(undefined, { year: 'numeric', month: 'short', day: 'numeric' }).format(new Date(iso));
	}
</script>

<div class="space-y-6">
	<div>
		<div style="font-family:'JetBrains Mono',monospace; font-size:10px; letter-spacing:.15em; color:#7c7cff; margin-bottom:8px; text-transform:uppercase;">
			── Permissions
		</div>
		<h1 class="text-2xl font-semibold" style="letter-spacing:-.02em;">Granted Rights</h1>
		<p class="text-sm mt-1" style="color:var(--c-muted);">Apps you've authorised to access your data. Revoke at any time.</p>
	</div>

	{#if loading}
		<div class="space-y-3">
			{#each [1,2,3] as _}
				<div class="rounded-xl p-4 animate-pulse" style="background:var(--c-surface); border:1px solid var(--c-border);">
					<div class="flex items-center justify-between">
						<div class="space-y-2">
							<div class="h-4 rounded w-32" style="background:var(--c-border-hi);"></div>
							<div class="h-3 rounded w-48" style="background:var(--c-border);"></div>
						</div>
						<div class="h-8 rounded w-20" style="background:var(--c-border-hi);"></div>
					</div>
				</div>
			{/each}
		</div>
	{:else if error}
		<div class="rounded-xl px-4 py-3 text-sm" style="background:rgba(248,113,113,.1); border:1px solid rgba(248,113,113,.3); color:#f87171; font-family:'JetBrains Mono',monospace; font-size:12px;">! {error}</div>
	{:else if rights.length === 0}
		<div class="rounded-xl p-12 text-center" style="background:var(--c-surface); border:1px solid var(--c-border);">
			<svg class="mx-auto w-10 h-10 mb-4" style="color:var(--c-border-hi);" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
				<path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
			</svg>
			<p class="font-medium" style="color:var(--c-muted);">No granted rights</p>
			<p class="text-sm mt-1" style="color:var(--c-border-hi); font-family:'JetBrains Mono',monospace; font-size:11px;">When you authorise an OAuth app, it appears here.</p>
		</div>
	{:else}
		<div class="space-y-2">
			{#each rights as right (right.client_id)}
				<div class="rounded-xl p-4 flex items-center justify-between gap-4" style="background:var(--c-surface); border:1px solid var(--c-border);">
					<div class="min-w-0">
						<p class="font-semibold text-sm">{right.app_name}</p>
						<p class="text-xs mt-0.5" style="font-family:'JetBrains Mono',monospace; color:var(--c-muted);">
							Granted {formatDate(right.granted_at)} · <span style="opacity:.6">{right.client_id}</span>
						</p>
					</div>
					<button
						onclick={() => revoke(right.client_id, right.app_name)}
						disabled={revoking === right.client_id}
						class="shrink-0 px-3 py-1.5 rounded-lg text-xs font-medium transition-colors"
						style="border:1px solid rgba(248,113,113,.3); color:#f87171; background:transparent; font-family:'JetBrains Mono',monospace; cursor:pointer; opacity:{revoking === right.client_id ? .5 : 1};"
						onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background='rgba(248,113,113,.1)'; }}
						onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background='transparent'; }}
					>
						{revoking === right.client_id ? 'Revoking…' : 'Revoke'}
					</button>
				</div>
			{/each}
		</div>
	{/if}
</div>
