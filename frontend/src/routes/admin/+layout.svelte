<script lang="ts">
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { auth } from '$lib/stores/auth';

	let { children } = $props();

	onMount(() => {
		let role = $auth.user?.role;
		if (!role && typeof localStorage !== 'undefined') {
			const raw = localStorage.getItem('rxforge_user');
			if (raw) {
				try {
					const stored = JSON.parse(raw);
					role = stored?.role;
				} catch {
					/* ignore */
				}
			}
		}
		if (role !== 'admin' && role !== 'superadmin') {
			goto('/dashboard');
		}
	});
</script>

<div class="min-h-screen bg-gray-100">
	<nav class="bg-white shadow-sm border-b border-red-200">
		<div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
			<div class="flex justify-between h-16">
				<div class="flex items-center gap-6">
					<span class="text-xl font-bold text-red-600">RxForge Admin</span>
					<a href="/admin/users" class="text-gray-600 hover:text-red-600 font-medium">Users</a>
					<a href="/admin/analytics" class="text-gray-600 hover:text-red-600 font-medium">Analytics</a>
					<a href="/dashboard" class="text-gray-600 hover:text-indigo-600 font-medium">Dashboard</a>
				</div>
				<div class="flex items-center gap-4">
					{#if $auth.user}
						<span class="text-gray-500 text-sm">{$auth.user.email}</span>
						<span class="px-2 py-0.5 bg-red-100 text-red-700 text-xs rounded-full font-medium">{$auth.user.role}</span>
					{/if}
					<button
						onclick={() => { $auth.logout(); goto('/login'); }}
						class="text-sm text-gray-600 hover:text-red-600"
					>Logout</button>
				</div>
			</div>
		</div>
	</nav>
	<main class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
		{@render children()}
	</main>
</div>
