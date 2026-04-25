<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	import { auth } from '$lib/stores/auth';
	import RxLogo from '$lib/components/RxLogo.svelte';

	let { children } = $props();
	let currentPath = $derived($page?.url?.pathname ?? '');

	onMount(() => {
		let role = $auth.user?.role;
		if (!role && typeof localStorage !== 'undefined') {
			try { role = JSON.parse(localStorage.getItem('rxforge_user') ?? '{}')?.role; } catch {}
		}
		if (role !== 'admin' && role !== 'superadmin') goto('/dashboard');
	});

	function isActive(href: string) {
		return currentPath === href || currentPath.startsWith(href + '/');
	}
</script>

<div class="min-h-screen" style="background:var(--c-bg); color:var(--c-text); font-family:'Space Grotesk',system-ui,sans-serif;">
	<nav style="position:sticky; top:0; z-index:20; background:color-mix(in srgb, var(--c-surface) 85%, transparent); backdrop-filter:blur(12px); border-bottom:1px solid #f872720a;">
		<!-- Admin accent top border -->
		<div style="height:2px; background:linear-gradient(90deg,#f87171,#fca5a5,transparent);"></div>
		<div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
			<div class="flex items-center justify-between h-14">
				<div class="flex items-center gap-1">
					<a href="/admin/users" class="mr-4 flex items-center gap-2">
						<RxLogo size={24} color="#eef0fa" accent="#f87171" />
						<span style="font-family:'JetBrains Mono',monospace; font-size:10px; letter-spacing:.12em; color:#f87171; text-transform:uppercase;">Admin</span>
					</a>

					{#each [{ href: '/admin/users', label: 'Users' }, { href: '/admin/analytics', label: 'Analytics' }] as link}
						<a
							href={link.href}
							class="px-3 py-1.5 rounded-lg text-sm font-medium transition-colors"
							style={isActive(link.href)
								? 'background:rgba(248,113,113,.12); color:#f87171; font-weight:600;'
								: 'color:var(--c-muted);'}
							onmouseenter={(e) => { if (!isActive(link.href)) { (e.currentTarget as HTMLElement).style.background='color-mix(in srgb,var(--c-text) 6%,transparent)'; (e.currentTarget as HTMLElement).style.color='var(--c-text)'; } }}
							onmouseleave={(e) => { if (!isActive(link.href)) { (e.currentTarget as HTMLElement).style.background=''; (e.currentTarget as HTMLElement).style.color='var(--c-muted)'; } }}
						>{link.label}</a>
					{/each}

					<a
						href="/dashboard"
						class="px-3 py-1.5 rounded-lg text-sm font-medium transition-colors"
						style="color:var(--c-muted);"
						onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background='color-mix(in srgb,var(--c-text) 6%,transparent)'; (e.currentTarget as HTMLElement).style.color='var(--c-text)'; }}
						onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background=''; (e.currentTarget as HTMLElement).style.color='var(--c-muted)'; }}
					>← Dashboard</a>
				</div>

				<div class="flex items-center gap-3">
					{#if $auth.user}
						<span class="hidden sm:block text-xs" style="font-family:'JetBrains Mono',monospace; color:var(--c-muted);">{$auth.user.email}</span>
						<span class="px-2 py-0.5 rounded text-xs font-medium" style="background:rgba(248,113,113,.12); color:#f87171; font-family:'JetBrains Mono',monospace; font-size:9px; letter-spacing:.08em;">{$auth.user.role?.toUpperCase()}</span>
					{/if}
					<button
						onclick={() => { auth.logout(); goto('/login'); }}
						class="text-xs px-3 py-1.5 rounded-lg transition-colors"
						style="border:1px solid var(--c-border); color:var(--c-muted); background:transparent; cursor:pointer;"
						onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.borderColor='#f87171'; (e.currentTarget as HTMLElement).style.color='#f87171'; }}
						onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.borderColor='var(--c-border)'; (e.currentTarget as HTMLElement).style.color='var(--c-muted)'; }}
					>Logout</button>
				</div>
			</div>
		</div>
	</nav>

	<main class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
		{@render children()}
	</main>
</div>
