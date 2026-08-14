<script lang="ts">
	import { onMount } from 'svelte';
	import { get } from 'svelte/store';
	import { api } from '$lib/api';
	import { toast } from '$lib/stores/toast';
	import { t } from '$lib/i18n';
	import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
	import DbBrowser from '$lib/components/DbBrowser.svelte';

	let users = $state<any[]>([]);
	let loading = $state(true);
	let search = $state('');
	let roleFilter = $state('all');
	let selectedUser = $state<any>(null);
	let slideoverOpen = $state(false);

	let confirmOpen = $state(false);
	let confirmTitle = $state('');
	let confirmMessage = $state('');
	let confirmAction = $state<() => void>(() => {});

	const ROLES = ['user', 'admin', 'superadmin'];
	const ALL_PERMISSIONS = ['read', 'write', 'delete', 'manage_users', 'manage_apps', 'view_analytics'];

	let editRole = $state('user');
	let editPermissions = $state<string[]>([]);
	let saving = $state(false);

	let userApps = $state<any[]>([]);
	let loadingApps = $state(false);
	let adminBrowseTarget = $state<{ app: any; userId: string } | null>(null);
	let activeTab = $state<'rights' | 'apps'>('rights');

	function openConfirm(title: string, message: string, action: () => void) {
		confirmTitle = title;
		confirmMessage = message;
		confirmAction = action;
		confirmOpen = true;
	}

	async function loadUsers() {
		loading = true;
		try {
			users = await api.admin.users.list();
		} catch (e: any) {
			toast.error('Failed to load users: ' + e.message);
		} finally {
			loading = false;
		}
	}

	async function openUser(user: any, tab: 'rights' | 'apps' = 'rights') {
		selectedUser = user;
		editRole = user.role ?? 'user';
		editPermissions = [...(user.permissions ?? [])];
		userApps = [];
		activeTab = tab;
		slideoverOpen = true;
		loadingApps = true;
		try {
			userApps = await api.admin.users.apps(user.id);
		} catch {
			userApps = [];
		} finally {
			loadingApps = false;
		}
	}

	async function saveRole() {
		if (!selectedUser) return;
		saving = true;
		try {
			await api.admin.users.updateRole(selectedUser.id, editRole);
			users = users.map(u => u.id === selectedUser.id ? { ...u, role: editRole } : u);
			selectedUser = { ...selectedUser, role: editRole };
			toast.success(get(t)('admin.userSaved'));
		} catch (e: any) {
			toast.error('Failed to update role: ' + e.message);
		} finally {
			saving = false;
		}
	}

	async function savePermissions() {
		if (!selectedUser) return;
		saving = true;
		try {
			await api.admin.users.updatePermissions(selectedUser.id, editPermissions);
			users = users.map(u => u.id === selectedUser.id ? { ...u, permissions: editPermissions } : u);
			selectedUser = { ...selectedUser, permissions: editPermissions };
			toast.success(get(t)('admin.userSaved'));
		} catch (e: any) {
			toast.error('Failed to update permissions: ' + e.message);
		} finally {
			saving = false;
		}
	}

	async function toggleLock() {
		if (!selectedUser) return;
		const locked = !selectedUser.locked;
		try {
			await api.admin.users.setLocked(selectedUser.id, locked);
			users = users.map(u => u.id === selectedUser.id ? { ...u, locked } : u);
			selectedUser = { ...selectedUser, locked };
			toast.success(locked ? 'Account locked.' : 'Account unlocked.');
		} catch (e: any) {
			toast.error('Failed to update account status: ' + e.message);
		}
	}

	function togglePermission(perm: string) {
		if (editPermissions.includes(perm)) {
			editPermissions = editPermissions.filter(p => p !== perm);
		} else {
			editPermissions = [...editPermissions, perm];
		}
	}

	const filteredUsers = $derived(
		users.filter(u => {
			const matchSearch =
				!search ||
				u.email?.toLowerCase().includes(search.toLowerCase()) ||
				u.id?.toLowerCase().includes(search.toLowerCase());
			const matchRole = roleFilter === 'all' || u.role === roleFilter;
			return matchSearch && matchRole;
		})
	);

	onMount(loadUsers);
</script>

<div class="space-y-6">
	<div class="flex items-center justify-between">
		<h1 class="text-2xl font-bold" style="color:var(--c-text);">{$t('admin.users')}</h1>
		<span class="text-sm" style="color:var(--c-muted);">{users.length} total users</span>
	</div>

	<!-- Filters -->
	<div class="flex gap-3 flex-wrap">
		<input
			type="search"
			bind:value={search}
			placeholder={$t('admin.search')}
			class="flex-1 min-w-48 px-4 py-2 rounded-lg text-sm outline-none"
			style="background:var(--c-surface); border:1px solid var(--c-border); color:var(--c-text);"
			onfocus={(e) => { (e.currentTarget as HTMLElement).style.borderColor='#f87171'; }}
			onblur={(e) => { (e.currentTarget as HTMLElement).style.borderColor='var(--c-border)'; }}
		/>
		<select
			bind:value={roleFilter}
			class="px-4 py-2 rounded-lg text-sm outline-none"
			style="background:var(--c-surface); border:1px solid var(--c-border); color:var(--c-text);"
		>
			<option value="all">{$t('admin.allRoles')}</option>
			{#each ROLES as r}
				<option value={r}>{r}</option>
			{/each}
		</select>
	</div>

	<!-- Table -->
	{#if loading}
		<div class="flex justify-center py-16">
			<div class="w-8 h-8 border-4 rounded-full animate-spin" style="border-color:rgba(248,113,113,.25); border-top-color:#f87171;"></div>
			<p class="sr-only">{$t('common.loading')}</p>
		</div>
	{:else}
		<div class="rounded-2xl overflow-hidden" style="background:var(--c-surface); border:1px solid var(--c-border);">
			<table class="w-full">
				<thead style="background:var(--c-surface-2); border-bottom:1px solid var(--c-border);">
					<tr>
						<th class="text-left px-6 py-3 text-xs font-medium uppercase tracking-wide" style="color:var(--c-muted);">Email</th>
						<th class="text-left px-6 py-3 text-xs font-medium uppercase tracking-wide" style="color:var(--c-muted);">Role</th>
						<th class="text-left px-6 py-3 text-xs font-medium uppercase tracking-wide" style="color:var(--c-muted);">Status</th>
						<th class="text-left px-6 py-3 text-xs font-medium uppercase tracking-wide" style="color:var(--c-muted);">Actions</th>
					</tr>
				</thead>
				<tbody>
					{#each filteredUsers as user (user.id)}
						<tr
							role="button"
							tabindex="0"
							onclick={() => openUser(user)}
							onkeydown={(e) => e.key === 'Enter' && openUser(user)}
							class="cursor-pointer transition-colors"
							style="border-top:1px solid var(--c-border);"
							onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background='rgba(255,255,255,.02)'; }}
							onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background=''; }}
						>
							<td class="px-6 py-4">
								<div class="font-medium" style="color:var(--c-text);">{user.email}</div>
								<div class="text-xs mt-0.5 font-mono" style="color:var(--c-muted);">{user.id}</div>
							</td>
							<td class="px-6 py-4">
								<span class="px-2 py-0.5 rounded-full text-xs font-medium" style="{
									user.role === 'superadmin' ? 'background:rgba(168,85,247,.15); color:#c084fc;' :
									user.role === 'admin' ? 'background:rgba(248,113,113,.15); color:#f87171;' :
									'background:var(--c-surface-2); color:var(--c-muted);'
								}">
									{user.role ?? 'user'}
								</span>
							</td>
							<td class="px-6 py-4">
								{#if user.locked}
									<span class="px-2 py-0.5 rounded-full text-xs font-medium" style="background:rgba(248,113,113,.15); color:#f87171;">Locked</span>
								{:else}
									<span class="px-2 py-0.5 rounded-full text-xs font-medium" style="background:rgba(74,222,128,.15); color:#4ade80;">Active</span>
								{/if}
							</td>
							<td class="px-6 py-4">
								<div class="flex items-center gap-2">
									<button
										onclick={(e) => { e.stopPropagation(); openUser(user, 'apps'); }}
										class="text-xs font-medium px-3 py-1.5 rounded-lg transition flex items-center gap-1.5"
										style="color:#34d399; border:1px solid rgba(52,211,153,.25); background:rgba(52,211,153,.06);"
										onmouseenter={(e) => { e.stopPropagation(); (e.currentTarget as HTMLElement).style.background='rgba(52,211,153,.12)'; }}
										onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background='rgba(52,211,153,.06)'; }}
									>
										<svg class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 12h18M3 6h18M3 18h18"/></svg>
										Apps
									</button>
									<button
										onclick={(e) => { e.stopPropagation(); openUser(user, 'rights'); }}
										class="text-xs font-medium px-3 py-1.5 rounded-lg transition"
										style="color:#f87171; border:1px solid rgba(248,113,113,.25); background:rgba(248,113,113,.06);"
										onmouseenter={(e) => { e.stopPropagation(); (e.currentTarget as HTMLElement).style.background='rgba(248,113,113,.12)'; }}
										onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background='rgba(248,113,113,.06)'; }}
									>Manage</button>
								</div>
							</td>
						</tr>
					{:else}
						<tr>
							<td colspan="4" class="px-6 py-12 text-center" style="color:var(--c-muted);">{$t('admin.noUsers')}</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{/if}
</div>

<!-- Slide-over -->
{#if slideoverOpen && selectedUser}
	<div
		class="fixed inset-0 z-40"
		style="background:rgba(0,0,0,.55);"
		onclick={() => { slideoverOpen = false; }}
		role="presentation"
	></div>
	<div class="fixed right-0 top-0 h-full w-full max-w-2xl z-50 flex flex-col shadow-2xl" style="background:var(--c-surface);">

		<!-- Header -->
		<div class="px-6 py-4 flex items-center justify-between shrink-0" style="border-bottom:1px solid var(--c-border);">
			<div class="flex items-center gap-3 min-w-0">
				<div class="w-9 h-9 rounded-full flex items-center justify-center shrink-0 text-sm font-semibold" style="background:rgba(248,113,113,.15); color:#f87171;">
					{selectedUser.email[0].toUpperCase()}
				</div>
				<div class="min-w-0">
					<h2 class="text-base font-semibold truncate" style="color:var(--c-text);">{selectedUser.email}</h2>
					<div class="flex items-center gap-2 mt-0.5">
						<span class="text-xs font-mono" style="color:var(--c-muted);">{selectedUser.id}</span>
						<span class="text-xs px-1.5 py-0.5 rounded-full font-medium" style="{
							selectedUser.role === 'superadmin' ? 'background:rgba(168,85,247,.15); color:#c084fc;' :
							selectedUser.role === 'admin' ? 'background:rgba(248,113,113,.15); color:#f87171;' :
							'background:var(--c-surface-2); color:var(--c-muted);'
						}">{selectedUser.role ?? 'user'}</span>
						{#if selectedUser.locked}
							<span class="text-xs px-1.5 py-0.5 rounded-full font-medium" style="background:rgba(248,113,113,.15); color:#f87171;">Locked</span>
						{/if}
					</div>
				</div>
			</div>
			<button onclick={() => { slideoverOpen = false; }} class="w-8 h-8 flex items-center justify-center rounded-lg transition shrink-0" style="color:var(--c-muted);"
				onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background='rgba(255,255,255,.06)'; (e.currentTarget as HTMLElement).style.color='var(--c-text)'; }}
				onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background='transparent'; (e.currentTarget as HTMLElement).style.color='var(--c-muted)'; }}
			>
				<svg class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12"/></svg>
			</button>
		</div>

		<!-- Tabs -->
		<div class="flex shrink-0 px-6" style="border-bottom:1px solid var(--c-border);">
			{#each [{ id: 'rights', label: 'Rights' }, { id: 'apps', label: 'Apps' + (userApps.length ? ` (${userApps.length})` : '') }] as tab}
				<button
					onclick={() => { activeTab = tab.id as 'rights' | 'apps'; }}
					class="px-4 py-3 text-sm font-medium transition relative"
					style="color:{activeTab === tab.id ? 'var(--c-text)' : 'var(--c-muted)'}; border-bottom:2px solid {activeTab === tab.id ? '#f87171' : 'transparent'}; margin-bottom:-1px;"
					onmouseenter={(e) => { if (activeTab !== tab.id) (e.currentTarget as HTMLElement).style.color='var(--c-text)'; }}
					onmouseleave={(e) => { if (activeTab !== tab.id) (e.currentTarget as HTMLElement).style.color='var(--c-muted)'; }}
				>{tab.label}</button>
			{/each}
		</div>

		<!-- Tab content -->
		<div class="flex-1 overflow-y-auto">

			<!-- ── Rights Tab ── -->
			{#if activeTab === 'rights'}
				<div class="p-6 space-y-6">

					<!-- Role -->
					<div class="rounded-xl p-5" style="background:var(--c-surface-2); border:1px solid var(--c-border);">
						<p class="text-xs font-semibold uppercase tracking-wide mb-3" style="color:var(--c-muted);">Role</p>
						<div class="flex gap-2 items-center">
							<select
								id="editRole"
								bind:value={editRole}
								class="flex-1 px-3 py-2 rounded-lg text-sm outline-none"
								style="background:var(--c-surface); border:1px solid var(--c-border); color:var(--c-text);"
							>
								{#each ROLES as r}
									<option value={r}>{r}</option>
								{/each}
							</select>
							<button
								onclick={saveRole}
								disabled={saving}
								class="px-4 py-2 text-sm font-semibold rounded-lg disabled:opacity-60 transition"
								style="background:#f87171; color:#fff;"
								onmouseenter={(e) => { if (!saving) (e.currentTarget as HTMLElement).style.background='#ef4444'; }}
								onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background='#f87171'; }}
							>{saving ? $t('common.saving') : $t('common.save')}</button>
						</div>
					</div>

					<!-- Permissions -->
					<div class="rounded-xl p-5" style="background:var(--c-surface-2); border:1px solid var(--c-border);">
						<p class="text-xs font-semibold uppercase tracking-wide mb-3" style="color:var(--c-muted);">Permissions</p>
						<div class="grid grid-cols-2 gap-2">
							{#each ALL_PERMISSIONS as perm}
								<label class="flex items-center gap-3 cursor-pointer rounded-lg px-3 py-2 transition" style="border:1px solid {editPermissions.includes(perm) ? 'rgba(248,113,113,.3)' : 'var(--c-border)'}; background:{editPermissions.includes(perm) ? 'rgba(248,113,113,.06)' : 'var(--c-surface)'};"
									onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.borderColor='rgba(248,113,113,.3)'; }}
									onmouseleave={(e) => { if (!editPermissions.includes(perm)) (e.currentTarget as HTMLElement).style.borderColor='var(--c-border)'; }}
								>
									<input
										type="checkbox"
										checked={editPermissions.includes(perm)}
										onchange={() => togglePermission(perm)}
										class="w-4 h-4 rounded accent-red-400"
									/>
									<span class="text-sm font-mono" style="color:var(--c-text);">{perm}</span>
								</label>
							{/each}
						</div>
						<button
							onclick={savePermissions}
							disabled={saving}
							class="mt-4 px-4 py-2 text-sm font-semibold rounded-lg disabled:opacity-60 transition"
							style="background:#f87171; color:#fff;"
							onmouseenter={(e) => { if (!saving) (e.currentTarget as HTMLElement).style.background='#ef4444'; }}
							onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background='#f87171'; }}
						>{saving ? $t('common.saving') : 'Save Permissions'}</button>
					</div>

					<!-- Account Status -->
					<div class="rounded-xl p-5" style="background:var(--c-surface-2); border:1px solid var(--c-border);">
						<p class="text-xs font-semibold uppercase tracking-wide mb-3" style="color:var(--c-muted);">Account Status</p>
						<div class="flex items-center justify-between">
							<div>
								<p class="text-sm font-medium" style="color:var(--c-text);">{selectedUser.locked ? 'Locked' : 'Active'}</p>
								<p class="text-xs mt-0.5" style="color:var(--c-muted);">{selectedUser.locked ? 'User cannot log in.' : 'User can log in normally.'}</p>
							</div>
							<button
								onclick={() => openConfirm(
									selectedUser.locked ? 'Unlock Account' : 'Lock Account',
									selectedUser.locked
										? `Allow ${selectedUser.email} to log in again?`
										: `Prevent ${selectedUser.email} from logging in?`,
									() => { toggleLock(); confirmOpen = false; }
								)}
								class="px-4 py-2 text-sm font-semibold rounded-lg transition"
								style="{selectedUser.locked ? 'background:#4ade80; color:#052e16;' : 'background:#f87171; color:#fff;'}"
								onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.opacity='.85'; }}
								onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.opacity='1'; }}
							>{selectedUser.locked ? 'Unlock' : 'Lock'}</button>
						</div>
					</div>

				</div>

			<!-- ── Apps Tab ── -->
			{:else if activeTab === 'apps'}
				<div class="p-6">
					{#if loadingApps}
						<div class="flex justify-center py-16">
							<div class="w-6 h-6 rounded-full border-2 animate-spin" style="border-color:#f87171; border-top-color:transparent;"></div>
						</div>
					{:else if userApps.length === 0}
						<div class="text-center py-16 rounded-xl" style="border:1px solid var(--c-border);">
							<svg class="w-10 h-10 mx-auto mb-3 opacity-20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/></svg>
							<p class="text-sm" style="color:var(--c-muted);">No apps found for this user.</p>
						</div>
					{:else}
						<div class="space-y-3">
							{#each userApps as app (app.id + '-' + app.relationship)}
								<div class="rounded-xl overflow-hidden" style="background:var(--c-surface-2); border:1px solid var(--c-border);">
									<!-- App header -->
									<div class="px-5 py-4">
										<div class="flex items-start justify-between gap-3">
											<div class="min-w-0">
												<div class="flex items-center gap-2 flex-wrap">
													<span class="font-semibold" style="color:var(--c-text);">{app.name}</span>
													<span class="text-xs px-2 py-0.5 rounded-full font-medium" style="{app.auth_type === 'token' ? 'background:rgba(251,191,36,.12); color:#fbbf24;' : 'background:rgba(124,124,255,.12); color:#7c7cff;'}">
														{app.auth_type === 'token' ? 'Token' : 'OAuth 2.0'}
													</span>
													{#if app.db_scope === 'shared'}
														<span class="text-xs px-2 py-0.5 rounded-full font-medium" style="background:rgba(248,113,113,.12); color:#f87171;">Shared DB</span>
													{/if}
													{#if app.relationship === 'consented'}
														<span class="text-xs px-2 py-0.5 rounded-full font-medium" style="background:rgba(52,211,153,.1); color:#34d399;">OAuth consented</span>
													{:else}
														<span class="text-xs px-2 py-0.5 rounded-full font-medium" style="background:rgba(251,191,36,.1); color:#fbbf24;">Owner</span>
													{/if}
												</div>
												<p class="text-xs mt-1 font-mono" style="color:var(--c-muted);">ID: {app.id}</p>
											</div>
											<button
												onclick={() => { adminBrowseTarget = { app, userId: selectedUser.id }; }}
												class="shrink-0 text-sm font-medium px-4 py-2 rounded-lg transition flex items-center gap-2"
												style="color:#34d399; border:1px solid rgba(52,211,153,.3); background:rgba(52,211,153,.06);"
												onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background='rgba(52,211,153,.14)'; }}
												onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background='rgba(52,211,153,.06)'; }}
											>
												<svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 12h18M3 6h18M3 18h18"/></svg>
												Browse
											</button>
										</div>
									</div>
								</div>
							{/each}
						</div>
					{/if}
				</div>
			{/if}
		</div>
	</div>
{/if}

{#if adminBrowseTarget}
	<DbBrowser
		app={adminBrowseTarget.app}
		userId={adminBrowseTarget.userId}
		onclose={() => { adminBrowseTarget = null; }}
	/>
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
