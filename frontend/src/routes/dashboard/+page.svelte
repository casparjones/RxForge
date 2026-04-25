<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import { auth } from '$lib/stores/auth';

	let stats = $state<{ last_login_at: string | null; app_count: number; granted_rights_count: number } | null>(null);
	let loading = $state(true);
	let error = $state('');

	onMount(async () => {
		try { stats = await api.me.stats(); }
		catch (e: any) { error = e.message; }
		finally { loading = false; }
	});

	function formatDate(iso: string | null) {
		if (!iso) return 'Never';
		return new Intl.DateTimeFormat(undefined, {
			year: 'numeric', month: 'short', day: 'numeric',
			hour: '2-digit', minute: '2-digit',
		}).format(new Date(iso));
	}
</script>

<div class="space-y-8">
	<div>
		<div style="font-family:'JetBrains Mono',monospace; font-size:10px; letter-spacing:.15em; color:#7c7cff; margin-bottom:8px; text-transform:uppercase;">
			── Overview
		</div>
		<h1 class="text-2xl font-semibold" style="letter-spacing:-.02em;">
			Welcome back{$auth.user?.email ? `, ${$auth.user.email.split('@')[0]}` : ''}.
		</h1>
		<p class="text-sm mt-1" style="color:var(--c-muted);">Here's a summary of your account.</p>
	</div>

	{#if loading}
		<div class="grid grid-cols-1 sm:grid-cols-3 gap-4">
			{#each [1,2,3] as _}
				<div class="rounded-xl p-6 animate-pulse" style="background:var(--c-surface); border:1px solid var(--c-border);">
					<div class="h-3 rounded w-24 mb-3" style="background:var(--c-border-hi);"></div>
					<div class="h-7 rounded w-16" style="background:var(--c-border-hi);"></div>
				</div>
			{/each}
		</div>
	{:else if error}
		<div class="rounded-xl px-4 py-3 text-sm" style="background:rgba(248,113,113,.1); border:1px solid rgba(248,113,113,.3); color:#f87171; font-family:'JetBrains Mono',monospace; font-size:12px;">! {error}</div>
	{:else if stats}
		<div class="grid grid-cols-1 sm:grid-cols-3 gap-4">
			<!-- Last login -->
			<div class="rounded-xl p-6" style="background:var(--c-surface); border:1px solid var(--c-border);">
				<p style="font-family:'JetBrains Mono',monospace; font-size:9px; letter-spacing:.12em; text-transform:uppercase; color:var(--c-muted); margin-bottom:8px;">Last Login</p>
				<p class="text-sm font-medium">{formatDate(stats.last_login_at)}</p>
			</div>

			<!-- Apps -->
			<a href="/dashboard/apps" class="block rounded-xl p-6 transition-colors group" style="background:var(--c-surface); border:1px solid var(--c-border); text-decoration:none; color:inherit;"
				onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.borderColor='#7c7cff66'; }}
				onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.borderColor='var(--c-border)'; }}>
				<p style="font-family:'JetBrains Mono',monospace; font-size:9px; letter-spacing:.12em; text-transform:uppercase; color:var(--c-muted); margin-bottom:8px;">Apps</p>
				<p class="text-3xl font-bold" style="color:#7c7cff;">{stats.app_count}</p>
				<p class="text-xs mt-1.5" style="color:var(--c-muted); font-family:'JetBrains Mono',monospace; font-size:10px;">Manage apps →</p>
			</a>

			<!-- Rights -->
			<a href="/dashboard/rights" class="block rounded-xl p-6 transition-colors" style="background:var(--c-surface); border:1px solid var(--c-border); text-decoration:none; color:inherit;"
				onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.borderColor='#7c7cff66'; }}
				onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.borderColor='var(--c-border)'; }}>
				<p style="font-family:'JetBrains Mono',monospace; font-size:9px; letter-spacing:.12em; text-transform:uppercase; color:var(--c-muted); margin-bottom:8px;">Granted Rights</p>
				<p class="text-3xl font-bold" style="color:#7c7cff;">{stats.granted_rights_count}</p>
				<p class="text-xs mt-1.5" style="color:var(--c-muted); font-family:'JetBrains Mono',monospace; font-size:10px;">Manage rights →</p>
			</a>
		</div>

		<!-- Quick actions -->
		<div class="rounded-xl p-6" style="background:var(--c-surface); border:1px solid var(--c-border);">
			<p style="font-family:'JetBrains Mono',monospace; font-size:9px; letter-spacing:.12em; text-transform:uppercase; color:var(--c-muted); margin-bottom:16px;">Quick Actions</p>
			<div class="flex flex-wrap gap-3">
				<a href="/dashboard/apps"
					class="inline-flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-semibold transition-colors"
					style="background:#7c7cff; color:#05050f;"
					onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background='#9090ff'; }}
					onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background='#7c7cff'; }}>
					<svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M12 5v14M5 12h14"/></svg>
					New App
				</a>
				<a href="/dashboard/rights"
					class="inline-flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-colors"
					style="border:1px solid var(--c-border); color:var(--c-muted);"
					onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.borderColor='var(--c-border-hi)'; (e.currentTarget as HTMLElement).style.color='var(--c-text)'; }}
					onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.borderColor='var(--c-border)'; (e.currentTarget as HTMLElement).style.color='var(--c-muted)'; }}>
					Manage Rights
				</a>
			</div>
		</div>
	{/if}
</div>
