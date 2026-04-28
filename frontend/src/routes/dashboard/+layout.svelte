<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	import { auth } from '$lib/stores/auth';
	import { theme } from '$lib/stores/theme';
	import { t, lang } from '$lib/i18n';
	import RxLogo from '$lib/components/RxLogo.svelte';

	let { children } = $props();
	let currentPath = $derived($page?.url?.pathname ?? '');

	onMount(() => {
		if (!$auth.token && typeof localStorage !== 'undefined') {
			if (!localStorage.getItem('rxforge_token')) goto('/login');
		} else if (!$auth.token) {
			goto('/login');
		}
	});

	function isActive(href: string) {
		return currentPath === href || (href !== '/dashboard' && currentPath.startsWith(href + '/'));
	}

	const navLinks = [
		{ href: '/dashboard',            labelKey: 'nav.dashboard' },
		{ href: '/dashboard/rights',     labelKey: 'nav.rights' },
		{ href: '/dashboard/apps',       labelKey: 'nav.apps' },
		{ href: '/dashboard/analytics',  labelKey: 'nav.analytics' },
	];
</script>

<div class="min-h-screen transition-colors"
	style="background:var(--c-bg); color:var(--c-text); font-family:'Space Grotesk',system-ui,sans-serif;">

	<!-- Top nav -->
	<nav style="
		position:sticky; top:0; z-index:20;
		background:color-mix(in srgb, var(--c-surface) 85%, transparent);
		backdrop-filter:blur(12px);
		border-bottom:1px solid var(--c-border);
	">
		<div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
			<div class="flex items-center justify-between h-14">
				<!-- Brand + nav links -->
				<div class="flex items-center gap-1">
					<a href="/dashboard" class="mr-4">
						<RxLogo
							size={26}
							color={$theme === 'dark' ? '#eef0fa' : '#0f172a'}
							accent="#7c7cff"
						/>
					</a>

					{#each navLinks as link}
						<a
							href={link.href}
							class="px-3 py-1.5 rounded-lg text-sm font-medium transition-colors"
							style={isActive(link.href)
								? 'background:color-mix(in srgb,#7c7cff 15%,transparent); color:#7c7cff; font-weight:600;'
								: 'color:var(--c-muted);'}
							onmouseenter={(e) => { if (!isActive(link.href)) (e.currentTarget as HTMLElement).style.background = 'color-mix(in srgb,var(--c-text) 6%,transparent)'; (e.currentTarget as HTMLElement).style.color = 'var(--c-text)'; }}
							onmouseleave={(e) => { if (!isActive(link.href)) { (e.currentTarget as HTMLElement).style.background = ''; (e.currentTarget as HTMLElement).style.color = 'var(--c-muted)'; } }}
						>{$t(link.labelKey)}</a>
					{/each}

					{#if $auth.user?.role === 'admin' || $auth.user?.role === 'superadmin'}
						<a
							href="/admin/users"
							class="px-3 py-1.5 rounded-lg text-sm font-medium transition-colors"
							style="color:#f87171;"
							onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = 'rgba(248,113,113,.1)'; }}
							onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = ''; }}
						>{$t('nav.admin')}</a>
					{/if}
				</div>

				<!-- Right controls -->
				<div class="flex items-center gap-2">
					{#if $auth.user}
						<span class="hidden sm:block text-xs truncate max-w-[160px]" style="color:var(--c-muted); font-family:'JetBrains Mono',monospace;">{$auth.user.email}</span>
					{/if}

					<!-- Theme toggle -->
					<button
						onclick={() => theme.toggle()}
						class="p-2 rounded-lg transition-colors"
						style="color:var(--c-muted); background:transparent;"
						onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = 'color-mix(in srgb,var(--c-text) 8%,transparent)'; }}
						onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = 'transparent'; }}
						title="Toggle theme"
					>
						{#if $theme === 'dark'}
							<!-- Sun -->
							<svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
								<circle cx="12" cy="12" r="5"/><path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42"/>
							</svg>
						{:else}
							<!-- Moon -->
							<svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
								<path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/>
							</svg>
						{/if}
					</button>

					<!-- Language toggle -->
					<button
						onclick={() => lang.update(l => l === 'en' ? 'de' : 'en')}
						class="text-xs px-2 py-1.5 rounded-lg transition-colors font-medium"
						style="border:1px solid var(--c-border); color:var(--c-muted); background:transparent;"
						onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = 'color-mix(in srgb,var(--c-text) 8%,transparent)'; (e.currentTarget as HTMLElement).style.color='var(--c-text)'; }}
						onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = 'transparent'; (e.currentTarget as HTMLElement).style.color='var(--c-muted)'; }}
					>{$lang === 'en' ? 'DE' : 'EN'}</button>

					<button
						onclick={() => { auth.logout(); goto('/login'); }}
						class="text-xs px-3 py-1.5 rounded-lg transition-colors"
						style="border:1px solid var(--c-border); color:var(--c-muted); background:transparent;"
						onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.borderColor='#f87171'; (e.currentTarget as HTMLElement).style.color='#f87171'; }}
						onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.borderColor='var(--c-border)'; (e.currentTarget as HTMLElement).style.color='var(--c-muted)'; }}
					>{$t('common.logout')}</button>
				</div>
			</div>
		</div>
	</nav>

	<main class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
		{@render children()}
	</main>
</div>
