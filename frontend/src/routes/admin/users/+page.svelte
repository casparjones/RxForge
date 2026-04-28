<script lang="ts">
	import { onMount } from 'svelte';
	import { get } from 'svelte/store';
	import { api } from '$lib/api';
	import { toast } from '$lib/stores/toast';
	import { t } from '$lib/i18n';
	import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';

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

	async function openUser(user: any) {
		selectedUser = user;
		editRole = user.role ?? 'user';
		editPermissions = [...(user.permissions ?? [])];
		userApps = [];
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
						<tr style="border-top:1px solid var(--c-border);">
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
								<button
									onclick={() => openUser(user)}
									class="text-sm font-medium transition-colors"
									style="color:#f87171;"
									onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.color='#fca5a5'; }}
									onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.color='#f87171'; }}
								>
									{$t('admin.impersonate')}
								</button>
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
		style="background:rgba(0,0,0,.5);"
		onclick={() => { slideoverOpen = false; }}
		role="presentation"
	></div>
	<div class="fixed right-0 top-0 h-full w-full max-w-md z-50 overflow-y-auto shadow-2xl" style="background:var(--c-surface);">
		<div class="p-6 flex items-center justify-between" style="border-bottom:1px solid var(--c-border);">
			<h2 class="text-lg font-semibold" style="color:var(--c-text);">{$t('admin.users')}</h2>
			<button onclick={() => { slideoverOpen = false; }} class="text-2xl leading-none" style="color:var(--c-muted);">&times;</button>
		</div>

		<div class="p-6 space-y-6">
			<div>
				<p class="text-sm" style="color:var(--c-muted);">Email</p>
				<p class="font-medium" style="color:var(--c-text);">{selectedUser.email}</p>
			</div>

			<!-- Role -->
			<div>
				<label for="editRole" class="block text-sm font-medium mb-2" style="color:var(--c-text);">{$t('admin.role')}</label>
				<div class="flex gap-2 items-center">
					<select
						id="editRole"
						bind:value={editRole}
						class="flex-1 px-3 py-2 rounded-lg text-sm outline-none"
						style="background:var(--c-surface-2); border:1px solid var(--c-border); color:var(--c-text);"
					>
						{#each ROLES as r}
							<option value={r}>{r}</option>
						{/each}
					</select>
					<button
						onclick={saveRole}
						disabled={saving}
						class="px-3 py-2 text-sm font-medium rounded-lg disabled:opacity-60 transition"
						style="background:#f87171; color:#fff;"
						onmouseenter={(e) => { if (!saving) (e.currentTarget as HTMLElement).style.background='#ef4444'; }}
						onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background='#f87171'; }}
					>{saving ? $t('common.saving') : $t('common.save')}</button>
				</div>
			</div>

			<!-- Permissions -->
			<div>
				<p class="text-sm font-medium mb-2" style="color:var(--c-text);">{$t('admin.permissions')}</p>
				<div class="space-y-2">
					{#each ALL_PERMISSIONS as perm}
						<label class="flex items-center gap-3 cursor-pointer">
							<input
								type="checkbox"
								checked={editPermissions.includes(perm)}
								onchange={() => togglePermission(perm)}
								class="w-4 h-4 rounded"
							/>
							<span class="text-sm font-mono" style="color:var(--c-text);">{perm}</span>
						</label>
					{/each}
				</div>
				<button
					onclick={savePermissions}
					disabled={saving}
					class="mt-3 px-3 py-2 text-sm font-medium rounded-lg disabled:opacity-60 transition"
					style="background:#f87171; color:#fff;"
					onmouseenter={(e) => { if (!saving) (e.currentTarget as HTMLElement).style.background='#ef4444'; }}
					onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background='#f87171'; }}
				>{saving ? $t('common.saving') : $t('admin.saveChanges')}</button>
			</div>

			<!-- Apps -->
			<div style="border-top:1px solid var(--c-border); padding-top:20px;">
				<p class="text-sm font-medium mb-3" style="color:var(--c-text);">Apps</p>
				{#if loadingApps}
					<p class="text-sm" style="color:var(--c-muted);">{$t('common.loading')}</p>
				{:else if userApps.length === 0}
					<p class="text-sm" style="color:var(--c-muted);">No apps registered.</p>
				{:else}
					<div class="space-y-2">
						{#each userApps as app (app.id)}
							<div class="rounded-lg px-4 py-3" style="background:var(--c-surface-2); border:1px solid var(--c-border);">
								<div class="flex items-center justify-between gap-2">
									<span class="font-medium text-sm" style="color:var(--c-text);">{app.name}</span>
									<span class="text-xs px-2 py-0.5 rounded-full" style="background:rgba(124,124,255,.12); color:#7c7cff; font-family:'JetBrains Mono',monospace;">{app.auth_type}</span>
								</div>
								<p class="text-xs mt-1 font-mono" style="color:var(--c-muted);">{app.id}</p>
							</div>
						{/each}
					</div>
				{/if}
			</div>

			<!-- Lock/Unlock -->
			<div style="border-top:1px solid var(--c-border); padding-top:20px;">
				<p class="text-sm font-medium mb-2" style="color:var(--c-text);">Account Status</p>
				<div class="flex items-center justify-between rounded-lg p-4" style="background:var(--c-surface-2);">
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
						class="px-4 py-2 text-sm font-medium rounded-lg transition"
						style="{selectedUser.locked ? 'background:#4ade80; color:#052e16;' : 'background:#f87171; color:#fff;'}"
					>
						{selectedUser.locked ? 'Unlock' : 'Lock'}
					</button>
				</div>
			</div>
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
