<script lang="ts">
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { auth } from '$lib/stores/auth';

	let { children } = $props();

	onMount(() => {
		if (!$auth.token && typeof localStorage !== 'undefined') {
			const token = localStorage.getItem('rxforge_token');
			if (!token) {
				goto('/login');
			}
		} else if (!$auth.token) {
			goto('/login');
		}
	});
</script>

<div class="min-h-screen bg-gray-100">
	<nav class="bg-white shadow-sm border-b border-gray-200">
		<div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
			<div class="flex justify-between h-16">
				<div class="flex items-center gap-6">
					<span class="text-xl font-bold text-indigo-600">RxForge</span>
					<a href="/dashboard/apps" class="text-gray-600 hover:text-indigo-600 font-medium">Apps</a>
					<a href="/dashboard/analytics" class="text-gray-600 hover:text-indigo-600 font-medium">Analytics</a>
					{#if $auth.user?.role === 'admin' || $auth.user?.role === 'superadmin'}
						<a href="/admin/users" class="text-gray-600 hover:text-red-600 font-medium">Admin</a>
					{/if}
				</div>
				<div class="flex items-center gap-4">
					{#if $auth.user}
						<span class="text-gray-500 text-sm">{$auth.user.email}</span>
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
