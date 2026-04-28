<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';
	import { auth } from '$lib/stores/auth';
	import { api } from '$lib/api';
	import { t } from '$lib/i18n';
	import RxLogo from '$lib/components/RxLogo.svelte';

	let clientId = $derived($page.url.searchParams.get('client_id') ?? '');
	let redirectUri = $derived($page.url.searchParams.get('redirect_uri') ?? '');
	let scope = $derived($page.url.searchParams.get('scope') ?? '');
	let stateParam = $derived($page.url.searchParams.get('state') ?? '');

	let appName = $state('');
	let loading = $state(true);
	let submitting = $state(false);
	let error = $state('');

	onMount(async () => {
		if (!$auth.token && typeof localStorage !== 'undefined') {
			const t = localStorage.getItem('rxforge_token');
			if (!t) {
				goto(`/login?return_to=${encodeURIComponent(window.location.href)}`);
				return;
			}
		}
		if (!clientId) { error = 'Missing client_id parameter'; loading = false; return; }
		try {
			const check = await api.oauth.consentCheck(clientId);
			if (check.consented) { await grant(); return; }
			const info = await api.oauth.clientInfo(clientId);
			appName = info.app_name;
		} catch (e: any) {
			error = e.message ?? 'Failed to load app info';
		} finally {
			loading = false;
		}
	});

	async function grant() {
		submitting = true;
		try {
			const res = await api.oauth.consentGrant({
				client_id: clientId,
				redirect_uri: redirectUri,
				scope: scope || undefined,
				state: stateParam || undefined,
			});
			window.location.href = res.redirect_url;
		} catch (e: any) {
			error = e.message ?? 'Failed to authorise';
			submitting = false;
		}
	}

	function deny() {
		const url = new URL(redirectUri);
		url.searchParams.set('error', 'access_denied');
		if (stateParam) url.searchParams.set('state', stateParam);
		window.location.href = url.toString();
	}
</script>

<!-- Always dark — it's a popup-style auth flow -->
<div
	class="min-h-screen flex items-center justify-center p-4"
	style="background:#0e0f1a; color:#eef0fa; font-family:'Space Grotesk',system-ui,sans-serif;"
>
	<!-- Corner ticks -->
	<div class="pointer-events-none fixed inset-0 z-0" aria-hidden="true">
		<span class="absolute top-8 left-8 w-7 h-7 border-t border-l" style="border-color:#2e3247"></span>
		<span class="absolute top-8 right-8 w-7 h-7 border-t border-r" style="border-color:#2e3247"></span>
		<span class="absolute bottom-8 left-8 w-7 h-7 border-b border-l" style="border-color:#2e3247"></span>
		<span class="absolute bottom-8 right-8 w-7 h-7 border-b border-r" style="border-color:#2e3247"></span>
	</div>

	<div class="w-full max-w-sm relative z-10">
		<!-- Header -->
		<div class="text-center mb-6">
			<RxLogo size={28} color="#eef0fa" accent="#7c7cff" class="justify-center mb-4" />
			<div style="font-family:'JetBrains Mono',monospace; font-size:10px; letter-spacing:.15em; color:#7c7cff; margin-bottom:8px; text-transform:uppercase;">── Authorise App</div>
		</div>

		{#if loading || submitting}
			<div class="rounded-xl p-10 text-center" style="background:#161829; border:1px solid #2e3247;">
				<div class="inline-block w-5 h-5 rounded-full border-2 border-t-transparent animate-spin mb-3" style="border-color:#7c7cff; border-top-color:transparent;"></div>
				<p style="font-family:'JetBrains Mono',monospace; font-size:12px; color:#8b8fa8;">{submitting ? $t('oauth.loading') : $t('oauth.loading')}</p>
			</div>
		{:else if error}
			<div class="rounded-xl p-6" style="background:#161829; border:1px solid #2e3247;">
				<div style="font-family:'JetBrains Mono',monospace; font-size:12px; color:#ff9ab0; background:#2a1422; border:1px solid #4a2034; border-radius:4px; padding:8px 12px;">! {$t('oauth.error')}: {error}</div>
			</div>
		{:else}
			<div class="rounded-xl p-8" style="background:#161829; border:1px solid #2e3247;">
				<!-- App info -->
				<div class="text-center mb-6">
					<div class="inline-flex items-center justify-center w-12 h-12 rounded-xl mb-3" style="background:rgba(124,124,255,.12); border:1px solid rgba(124,124,255,.2);">
						<svg class="w-6 h-6" style="color:#7c7cff;" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
							<path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
						</svg>
					</div>
					<p style="font-size:15px; color:#eef0fa;">
						{$t('oauth.title', { name: appName })}
					</p>
					{#if scope}
						<p class="mt-1" style="font-family:'JetBrains Mono',monospace; font-size:11px; color:#8b8fa8;">Scope: {scope}</p>
					{/if}
				</div>

				<!-- Permission list -->
				<div class="rounded-lg p-4 mb-5 space-y-2" style="background:#0e0f1a; border:1px solid #22253a;">
					{#each ['Read and write your synced app data', 'Access scoped to this app only'] as perm}
						<div class="flex items-center gap-3 text-sm" style="color:#8b8fa8;">
							<svg class="w-4 h-4 shrink-0" style="color:#4ade80;" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="20 6 9 17 4 12"/></svg>
							{perm}
						</div>
					{/each}
				</div>

				<!-- Buttons -->
				<div class="flex gap-3">
					<button
						onclick={deny}
						class="flex-1 py-2.5 rounded-lg text-sm font-medium transition-colors"
						style="border:1px solid #2e3247; color:#8b8fa8; background:transparent; font-family:'Space Grotesk',sans-serif; cursor:pointer;"
						onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.borderColor='#3e4257'; (e.currentTarget as HTMLElement).style.color='#eef0fa'; }}
						onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.borderColor='#2e3247'; (e.currentTarget as HTMLElement).style.color='#8b8fa8'; }}
					>{$t('oauth.deny')}</button>
					<button
						onclick={grant}
						class="flex-1 py-2.5 rounded-lg text-sm font-semibold transition-colors"
						style="background:#7c7cff; color:#05050f; border:none; font-family:'Space Grotesk',sans-serif; cursor:pointer;"
						onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background='#9090ff'; }}
						onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background='#7c7cff'; }}
					>{$t('oauth.allow')}</button>
				</div>

				<p class="text-center mt-4" style="font-family:'JetBrains Mono',monospace; font-size:10px; color:#4a4f6a;">
					Signed in as {$auth.user?.email ?? '…'} ·
					<a href="/dashboard/rights" style="color:#7c7cff; text-decoration:none;">manage rights</a>
				</p>
			</div>
		{/if}
	</div>
</div>
