<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import { toast } from '$lib/stores/toast';
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

	function openUser(user: any) {
		selectedUser = user;
		editRole = user.role ?? 'user';
		editPermissions = [...(user.permissions ?? [])];
		slideoverOpen = true;
	}

	async function saveRole() {
		if (!selectedUser) return;
		saving = true;
		try {
			await api.admin.users.updateRole(selectedUser.id, editRole);
			users = users.map(u => u.id === selectedUser.id ? { ...u, role: editRole } : u);
			selectedUser = { ...selectedUser, role: editRole };
			toast.success('Role updated.');
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
			toast.success('Permissions updated.');
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
		<h1 class="text-2xl font-bold text-gray-900">User Management</h1>
		<span class="text-sm text-gray-500">{users.length} total users</span>
	</div>

	<!-- Filters -->
	<div class="flex gap-3 flex-wrap">
		<input
			type="search"
			bind:value={search}
			placeholder="Search by email or ID…"
			class="px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-red-500 outline-none text-sm flex-1 min-w-48"
		/>
		<select
			bind:value={roleFilter}
			class="px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-red-500 outline-none text-sm bg-white"
		>
			<option value="all">All Roles</option>
			{#each ROLES as r}
				<option value={r}>{r}</option>
			{/each}
		</select>
	</div>

	<!-- Table -->
	{#if loading}
		<div class="flex justify-center py-16">
			<div class="w-8 h-8 border-4 border-red-200 border-t-red-600 rounded-full animate-spin"></div>
		</div>
	{:else}
		<div class="bg-white rounded-2xl border border-gray-200 shadow-sm overflow-hidden">
			<table class="w-full">
				<thead class="bg-gray-50 border-b border-gray-200">
					<tr>
						<th class="text-left px-6 py-3 text-xs font-medium text-gray-500 uppercase tracking-wide">Email</th>
						<th class="text-left px-6 py-3 text-xs font-medium text-gray-500 uppercase tracking-wide">Role</th>
						<th class="text-left px-6 py-3 text-xs font-medium text-gray-500 uppercase tracking-wide">Status</th>
						<th class="text-left px-6 py-3 text-xs font-medium text-gray-500 uppercase tracking-wide">Actions</th>
					</tr>
				</thead>
				<tbody class="divide-y divide-gray-100">
					{#each filteredUsers as user (user.id)}
						<tr class="hover:bg-gray-50 transition">
							<td class="px-6 py-4">
								<div class="font-medium text-gray-900">{user.email}</div>
								<div class="text-xs text-gray-400 font-mono mt-0.5">{user.id}</div>
							</td>
							<td class="px-6 py-4">
								<span class="px-2 py-0.5 rounded-full text-xs font-medium {
									user.role === 'superadmin' ? 'bg-purple-100 text-purple-700' :
									user.role === 'admin' ? 'bg-red-100 text-red-700' :
									'bg-gray-100 text-gray-600'
								}">
									{user.role ?? 'user'}
								</span>
							</td>
							<td class="px-6 py-4">
								{#if user.locked}
									<span class="px-2 py-0.5 rounded-full text-xs font-medium bg-red-100 text-red-700">Locked</span>
								{:else}
									<span class="px-2 py-0.5 rounded-full text-xs font-medium bg-green-100 text-green-700">Active</span>
								{/if}
							</td>
							<td class="px-6 py-4">
								<button
									onclick={() => openUser(user)}
									class="text-sm text-red-600 hover:text-red-800 font-medium"
								>
									Manage
								</button>
							</td>
						</tr>
					{:else}
						<tr>
							<td colspan="4" class="px-6 py-12 text-center text-gray-400">No users found</td>
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
		class="fixed inset-0 bg-black/40 z-40"
		onclick={() => { slideoverOpen = false; }}
		role="presentation"
	></div>
	<div class="fixed right-0 top-0 h-full w-full max-w-md bg-white shadow-2xl z-50 overflow-y-auto">
		<div class="p-6 border-b border-gray-200 flex items-center justify-between">
			<h2 class="text-lg font-semibold text-gray-900">Manage User</h2>
			<button onclick={() => { slideoverOpen = false; }} class="text-gray-400 hover:text-gray-600 text-2xl leading-none">&times;</button>
		</div>

		<div class="p-6 space-y-6">
			<div>
				<p class="text-sm text-gray-500">Email</p>
				<p class="font-medium text-gray-900">{selectedUser.email}</p>
			</div>

			<!-- Role -->
			<div>
				<label for="editRole" class="block text-sm font-medium text-gray-700 mb-2">Role</label>
				<div class="flex gap-2 items-center">
					<select
						id="editRole"
						bind:value={editRole}
						class="flex-1 px-3 py-2 border border-gray-300 rounded-lg text-sm bg-white focus:ring-2 focus:ring-red-500 outline-none"
					>
						{#each ROLES as r}
							<option value={r}>{r}</option>
						{/each}
					</select>
					<button
						onclick={saveRole}
						disabled={saving}
						class="px-3 py-2 bg-red-600 hover:bg-red-700 text-white text-sm font-medium rounded-lg disabled:opacity-60 transition"
					>Save</button>
				</div>
			</div>

			<!-- Permissions -->
			<div>
				<p class="block text-sm font-medium text-gray-700 mb-2">Permissions</p>
				<div class="space-y-2">
					{#each ALL_PERMISSIONS as perm}
						<label class="flex items-center gap-3 cursor-pointer">
							<input
								type="checkbox"
								checked={editPermissions.includes(perm)}
								onchange={() => togglePermission(perm)}
								class="w-4 h-4 text-red-600 rounded border-gray-300"
							/>
							<span class="text-sm text-gray-700 font-mono">{perm}</span>
						</label>
					{/each}
				</div>
				<button
					onclick={savePermissions}
					disabled={saving}
					class="mt-3 px-3 py-2 bg-red-600 hover:bg-red-700 text-white text-sm font-medium rounded-lg disabled:opacity-60 transition"
				>Save Permissions</button>
			</div>

			<!-- Lock/Unlock -->
			<div class="border-t border-gray-100 pt-5">
				<p class="block text-sm font-medium text-gray-700 mb-2">Account Status</p>
				<div class="flex items-center justify-between bg-gray-50 rounded-lg p-4">
					<div>
						<p class="text-sm font-medium text-gray-900">{selectedUser.locked ? 'Locked' : 'Active'}</p>
						<p class="text-xs text-gray-500 mt-0.5">{selectedUser.locked ? 'User cannot log in.' : 'User can log in normally.'}</p>
					</div>
					<button
						onclick={() => openConfirm(
							selectedUser.locked ? 'Unlock Account' : 'Lock Account',
							selectedUser.locked
								? `Allow ${selectedUser.email} to log in again?`
								: `Prevent ${selectedUser.email} from logging in?`,
							() => { toggleLock(); confirmOpen = false; }
						)}
						class="px-4 py-2 text-sm font-medium rounded-lg transition {selectedUser.locked
							? 'bg-green-600 hover:bg-green-700 text-white'
							: 'bg-red-600 hover:bg-red-700 text-white'}"
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
