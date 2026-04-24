<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import { toast } from '$lib/stores/toast';
	import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';

	let apps = $state<any[]>([]);
	let loading = $state(true);
	let showCreateModal = $state(false);
	let newAppName = $state('');
	let newAppRedirects = $state('');
	let creating = $state(false);

	// Expanded app state
	let expandedApp = $state<string | null>(null);
	let secretVisible = $state<Record<string, boolean>>({});
	let appStats = $state<Record<string, any>>({});

	// Confirm dialog state
	let confirmOpen = $state(false);
	let confirmTitle = $state('');
	let confirmMessage = $state('');
	let confirmAction = $state<() => void>(() => {});

	function openConfirm(title: string, message: string, action: () => void) {
		confirmTitle = title;
		confirmMessage = message;
		confirmAction = action;
		confirmOpen = true;
	}

	async function loadApps() {
		loading = true;
		try {
			apps = await api.apps.list();
		} catch (e: any) {
			toast.error('Failed to load apps: ' + e.message);
		} finally {
			loading = false;
		}
	}

	async function loadStats(appId: string) {
		try {
			appStats[appId] = await api.apps.getStats(appId);
		} catch {
			appStats[appId] = null;
		}
	}

	async function createApp() {
		if (!newAppName.trim()) {
			toast.error('App name is required.');
			return;
		}
		creating = true;
		try {
			const redirectUris = newAppRedirects.split('\n').map(s => s.trim()).filter(Boolean);
			const app = await api.apps.create({ name: newAppName.trim(), redirect_uris: redirectUris });
			apps = [...apps, app];
			showCreateModal = false;
			newAppName = '';
			newAppRedirects = '';
			toast.success('App created successfully!');
		} catch (e: any) {
			toast.error('Failed to create app: ' + e.message);
		} finally {
			creating = false;
		}
	}

	async function deleteApp(id: string) {
		try {
			await api.apps.delete(id);
			apps = apps.filter(a => a.id !== id);
			toast.success('App deleted.');
		} catch (e: any) {
			toast.error('Failed to delete app: ' + e.message);
		}
	}

	async function regenerateSecret(id: string) {
		try {
			const res = await api.apps.regenerateSecret(id);
			apps = apps.map(a => a.id === id ? { ...a, client_secret: res.client_secret } : a);
			toast.success('Secret regenerated!');
		} catch (e: any) {
			toast.error('Failed to regenerate secret: ' + e.message);
		}
	}

	function copyToClipboard(text: string, label: string) {
		navigator.clipboard.writeText(text).then(() => {
			toast.success(`${label} copied to clipboard!`);
		});
	}

	function toggleExpand(id: string) {
		if (expandedApp === id) {
			expandedApp = null;
		} else {
			expandedApp = id;
			loadStats(id);
		}
	}

	onMount(loadApps);
</script>

<div class="space-y-6">
	<div class="flex items-center justify-between">
		<h1 class="text-2xl font-bold text-gray-900">My Apps</h1>
		<button
			onclick={() => { showCreateModal = true; }}
			class="bg-indigo-600 hover:bg-indigo-700 text-white font-medium px-4 py-2 rounded-lg transition"
		>
			+ New App
		</button>
	</div>

	{#if loading}
		<div class="flex justify-center py-16">
			<div class="w-8 h-8 border-4 border-indigo-200 border-t-indigo-600 rounded-full animate-spin"></div>
		</div>
	{:else if apps.length === 0}
		<div class="text-center py-16 bg-white rounded-2xl border border-gray-200">
			<p class="text-gray-500 mb-4">No apps yet. Create your first app!</p>
			<button
				onclick={() => { showCreateModal = true; }}
				class="bg-indigo-600 hover:bg-indigo-700 text-white font-medium px-4 py-2 rounded-lg transition"
			>
				Create App
			</button>
		</div>
	{:else}
		<div class="grid gap-4">
			{#each apps as app (app.id)}
				<div class="bg-white rounded-2xl border border-gray-200 shadow-sm overflow-hidden">
					<div class="p-6">
						<div class="flex items-start justify-between">
							<div>
								<h2 class="text-lg font-semibold text-gray-900">{app.name}</h2>
								<p class="text-sm text-gray-400 mt-0.5">ID: {app.id}</p>
							</div>
							<div class="flex gap-2">
								<button
									onclick={() => toggleExpand(app.id)}
									class="text-sm text-indigo-600 hover:text-indigo-800 font-medium"
								>
									{expandedApp === app.id ? 'Hide details' : 'Show details'}
								</button>
								<button
									onclick={() => openConfirm('Delete App', `Are you sure you want to delete "${app.name}"? This action cannot be undone.`, () => { deleteApp(app.id); confirmOpen = false; })}
									class="text-sm text-red-600 hover:text-red-800 font-medium ml-2"
								>
									Delete
								</button>
							</div>
						</div>
					</div>

					{#if expandedApp === app.id}
						<div class="border-t border-gray-100 px-6 py-5 bg-gray-50 space-y-4">
							<!-- Client ID -->
							<div>
								<p class="block text-xs font-medium text-gray-500 mb-1 uppercase tracking-wide">Client ID</p>
								<div class="flex items-center gap-2">
									<code class="flex-1 text-sm bg-white border border-gray-200 rounded px-3 py-1.5 font-mono">{app.client_id}</code>
									<button
										onclick={() => copyToClipboard(app.client_id, 'Client ID')}
										class="text-xs text-indigo-600 hover:text-indigo-800 border border-indigo-200 rounded px-2 py-1.5 bg-white"
									>Copy</button>
								</div>
							</div>

							<!-- Client Secret -->
							<div>
								<p class="block text-xs font-medium text-gray-500 mb-1 uppercase tracking-wide">Client Secret</p>
								<div class="flex items-center gap-2">
									<code class="flex-1 text-sm bg-white border border-gray-200 rounded px-3 py-1.5 font-mono">
										{secretVisible[app.id] ? app.client_secret : '••••••••••••••••'}
									</code>
									<button
										onclick={() => { secretVisible = { ...secretVisible, [app.id]: !secretVisible[app.id] }; }}
										class="text-xs border border-gray-200 rounded px-2 py-1.5 bg-white text-gray-600"
										title={secretVisible[app.id] ? 'Hide' : 'Show'}
									>
										{secretVisible[app.id] ? 'Hide' : 'Show'}
									</button>
									<button
										onclick={() => copyToClipboard(app.client_secret, 'Client Secret')}
										class="text-xs text-indigo-600 hover:text-indigo-800 border border-indigo-200 rounded px-2 py-1.5 bg-white"
									>Copy</button>
									<button
										onclick={() => openConfirm('Regenerate Secret', 'This will invalidate the current secret. Continue?', () => { regenerateSecret(app.id); confirmOpen = false; })}
										class="text-xs text-orange-600 hover:text-orange-800 border border-orange-200 rounded px-2 py-1.5 bg-white"
									>Regenerate</button>
								</div>
							</div>

							<!-- Stats -->
							<div>
								<p class="block text-xs font-medium text-gray-500 mb-2 uppercase tracking-wide">Statistics</p>
								{#if appStats[app.id] === undefined}
									<p class="text-sm text-gray-400">Loading stats…</p>
								{:else if appStats[app.id] === null}
									<p class="text-sm text-gray-400">Stats unavailable</p>
								{:else}
									<div class="grid grid-cols-3 gap-3">
										<div class="bg-white border border-gray-200 rounded-lg p-3 text-center">
											<p class="text-2xl font-bold text-indigo-600">{appStats[app.id].requests_today ?? 0}</p>
											<p class="text-xs text-gray-500 mt-0.5">Today</p>
										</div>
										<div class="bg-white border border-gray-200 rounded-lg p-3 text-center">
											<p class="text-2xl font-bold text-indigo-600">{appStats[app.id].requests_7d ?? 0}</p>
											<p class="text-xs text-gray-500 mt-0.5">Last 7 days</p>
										</div>
										<div class="bg-white border border-gray-200 rounded-lg p-3 text-center">
											<p class="text-2xl font-bold text-indigo-600">{appStats[app.id].requests_30d ?? 0}</p>
											<p class="text-xs text-gray-500 mt-0.5">Last 30 days</p>
										</div>
									</div>
								{/if}
							</div>
						</div>
					{/if}
				</div>
			{/each}
		</div>
	{/if}
</div>

<!-- Create App Modal -->
{#if showCreateModal}
	<div
		class="fixed inset-0 bg-black/50 z-40 flex items-center justify-center p-4"
		onclick={() => { showCreateModal = false; }}
		role="presentation"
	>
		<div
			class="bg-white rounded-2xl shadow-xl max-w-md w-full p-6"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
			role="dialog"
			tabindex="-1"
			aria-modal="true"
		>
			<h2 class="text-xl font-semibold text-gray-900 mb-5">Create New App</h2>

			<form onsubmit={(e) => { e.preventDefault(); createApp(); }} class="space-y-4">
				<div>
					<label for="appName" class="block text-sm font-medium text-gray-700 mb-1">App Name</label>
					<input
						id="appName"
						type="text"
						bind:value={newAppName}
						required
						class="w-full px-4 py-2.5 border border-gray-300 rounded-lg focus:ring-2 focus:ring-indigo-500 outline-none"
						placeholder="My App"
					/>
				</div>

				<div>
					<label for="redirectUris" class="block text-sm font-medium text-gray-700 mb-1">
						Redirect URIs
						<span class="text-gray-400 font-normal">(one per line)</span>
					</label>
					<textarea
						id="redirectUris"
						bind:value={newAppRedirects}
						rows="3"
						class="w-full px-4 py-2.5 border border-gray-300 rounded-lg focus:ring-2 focus:ring-indigo-500 outline-none resize-none"
						placeholder="https://myapp.com/callback"
					></textarea>
				</div>

				<div class="flex gap-3 justify-end pt-2">
					<button
						type="button"
						onclick={() => { showCreateModal = false; }}
						class="px-4 py-2 text-sm font-medium text-gray-700 border border-gray-300 rounded-lg hover:bg-gray-50 transition"
					>Cancel</button>
					<button
						type="submit"
						disabled={creating}
						class="px-4 py-2 text-sm font-medium bg-indigo-600 hover:bg-indigo-700 disabled:opacity-60 text-white rounded-lg transition"
					>
						{creating ? 'Creating…' : 'Create App'}
					</button>
				</div>
			</form>
		</div>
	</div>
{/if}

<ConfirmDialog
	open={confirmOpen}
	title={confirmTitle}
	message={confirmMessage}
	confirmLabel="Confirm"
	destructive={true}
	onConfirm={confirmAction}
	onCancel={() => { confirmOpen = false; }}
/>
