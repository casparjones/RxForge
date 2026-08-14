<script lang="ts">
	import { api } from '$lib/api';
	import { toast } from '$lib/stores/toast';
	import { get } from 'svelte/store';
	import { t } from '$lib/i18n';
	import CodeEditor from './CodeEditor.svelte';
	import ConfirmDialog from './ConfirmDialog.svelte';

	interface Props {
		app: any;
		userId?: string;
		onclose: () => void;
	}

	let { app, userId, onclose }: Props = $props();

	// List state
	let docs = $state<any[]>([]);
	let total = $state(0);
	let page = $state(1);
	let pages = $state(1);
	const PER_PAGE = 20;
	let loading = $state(false);
	let listError = $state('');

	// Search & filter state
	let searchInput = $state('');
	let search = $state('');
	let deletedFilter = $state<'active' | 'deleted' | 'all'>('active');
	let searchTimer: ReturnType<typeof setTimeout> | undefined;

	function isDeleted(doc: any): boolean {
		return doc?._deleted === true;
	}

	function applySearch() {
		clearTimeout(searchTimer);
		searchTimer = setTimeout(() => {
			search = searchInput.trim();
			loadPage(1);
		}, 300);
	}

	function setFilter(f: 'active' | 'deleted' | 'all') {
		if (deletedFilter === f) return;
		deletedFilter = f;
		selectedDoc = null;
		loadPage(1);
	}

	// Multi-select for bulk delete / purge.
	// In the "deleted" view the selection targets tombstones for permanent removal (purge);
	// in other views it targets live documents for a (soft) delete.
	let selectedIds = $state<string[]>([]);
	let purgeMode = $derived(deletedFilter === 'deleted');
	let selectableIds = $derived(
		purgeMode ? docs.map(d => d._id) : docs.filter(d => !isDeleted(d)).map(d => d._id)
	);
	let allSelected = $derived(selectableIds.length > 0 && selectableIds.every(id => selectedIds.includes(id)));

	function toggleSelect(id: string) {
		selectedIds = selectedIds.includes(id) ? selectedIds.filter(x => x !== id) : [...selectedIds, id];
	}
	function toggleSelectAll() {
		selectedIds = allSelected ? [] : [...selectableIds];
	}
	function clearSelection() { selectedIds = []; }

	// Selected document
	let selectedDoc = $state<any | null>(null);
	let editJson = $state('');
	let saving = $state(false);
	let saveError = $state('');

	// Confirm dialog
	let confirmOpen = $state(false);
	let confirmTitle = $state('');
	let confirmMessage = $state('');
	let confirmAction = $state<() => void>(() => {});

	function openConfirm(title: string, message: string, action: () => void) {
		confirmTitle = title; confirmMessage = message; confirmAction = action; confirmOpen = true;
	}

	async function loadPage(p: number) {
		loading = true; listError = ''; selectedIds = [];
		try {
			let res;
			if (userId) {
				res = await api.admin.users.db.list(userId, app.id, p, PER_PAGE, search, deletedFilter);
			} else {
				res = await api.apps.db.list(app.id, p, PER_PAGE, search, deletedFilter);
			}
			docs = res.docs; total = res.total; page = res.page; pages = res.pages;
		} catch (e: any) {
			listError = e.message;
		} finally {
			loading = false;
		}
	}

	function selectDoc(doc: any) {
		selectedDoc = doc;
		editJson = JSON.stringify(doc, null, 2);
		saveError = '';
	}

	async function saveDoc() {
		if (!selectedDoc || userId) return;
		saving = true; saveError = '';
		try {
			let parsed: any;
			try { parsed = JSON.parse(editJson); } catch { saveError = 'Invalid JSON.'; saving = false; return; }
			const res = await api.apps.db.updateDoc(app.id, selectedDoc._id, parsed);
			const updated = { ...parsed, _rev: res.rev ?? parsed._rev };
			selectedDoc = updated;
			editJson = JSON.stringify(updated, null, 2);
			docs = docs.map(d => d._id === updated._id ? updated : d);
			toast.success(get(t)('db.documentSaved'));
		} catch (e: any) {
			saveError = e.message;
		} finally {
			saving = false;
		}
	}

	async function deleteDoc(doc: any) {
		try {
			if (userId) {
				await api.admin.users.db.deleteDoc(userId, app.id, doc._id, doc._rev);
			} else {
				await api.apps.db.deleteDoc(app.id, doc._id, doc._rev);
			}
			// When the active-only filter is applied the tombstone drops out of view;
			// with 'deleted'/'all' filters the list is refreshed to reflect the new state.
			if (deletedFilter === 'active') {
				docs = docs.filter(d => d._id !== doc._id);
				total = Math.max(0, total - 1);
				if (selectedDoc?._id === doc._id) selectedDoc = null;
			} else {
				if (selectedDoc?._id === doc._id) selectedDoc = null;
				await loadPage(page);
			}
			toast.success(get(t)('db.documentDeleted'));
		} catch (e: any) {
			toast.error('Error: ' + e.message);
		}
	}

	async function deleteAll() {
		try {
			const res = userId
				? await api.admin.users.db.deleteAll(userId, app.id)
				: await api.apps.db.deleteAll(app.id);
			docs = []; total = 0; page = 1; pages = 1; selectedDoc = null;
			toast.success(get(t)('db.documentsDeleted', { n: res.deleted }));
		} catch (e: any) {
			toast.error('Error: ' + e.message);
		}
	}

	async function deleteSelected() {
		const targets = docs.filter(d => selectedIds.includes(d._id) && !isDeleted(d));
		let ok = 0;
		for (const doc of targets) {
			try {
				if (userId) {
					await api.admin.users.db.deleteDoc(userId, app.id, doc._id, doc._rev);
				} else {
					await api.apps.db.deleteDoc(app.id, doc._id, doc._rev);
				}
				ok++;
			} catch (e: any) {
				toast.error('Error: ' + e.message);
			}
		}
		if (selectedDoc && targets.some(d => d._id === selectedDoc._id)) selectedDoc = null;
		selectedIds = [];
		await loadPage(page);
		toast.success(get(t)('db.documentsDeleted', { n: ok }));
	}

	async function purgeSelected() {
		const ids = docs.filter(d => selectedIds.includes(d._id)).map(d => d._id);
		if (!ids.length) return;
		try {
			const res = userId
				? await api.admin.users.db.purge(userId, app.id, ids)
				: await api.apps.db.purge(app.id, ids);
			if (selectedDoc && ids.includes(selectedDoc._id)) selectedDoc = null;
			selectedIds = [];
			await loadPage(page);
			toast.success(get(t)('db.documentsPurged', { n: res.purged }));
		} catch (e: any) {
			toast.error('Error: ' + e.message);
		}
	}

	async function purgeAllDeleted() {
		try {
			const res = userId
				? await api.admin.users.db.purgeDeleted(userId, app.id)
				: await api.apps.db.purgeDeleted(app.id);
			selectedDoc = null;
			selectedIds = [];
			await loadPage(1);
			toast.success(get(t)('db.documentsPurged', { n: res.purged }));
		} catch (e: any) {
			toast.error('Error: ' + e.message);
		}
	}

	function preview(doc: any): string {
		const skip = new Set(['_id', '_rev', '_deleted']);
		const entries = Object.entries(doc).filter(([k]) => !skip.has(k)).slice(0, 2);
		if (!entries.length) return '—';
		return entries.map(([k, v]) => {
			const val = typeof v === 'object' ? JSON.stringify(v) : String(v);
			return `${k}: ${val.length > 24 ? val.slice(0, 24) + '…' : val}`;
		}).join('  ·  ');
	}

	$effect(() => { loadPage(1); });
</script>

<!-- Full-screen overlay -->
<div class="fixed inset-0 z-50 flex flex-col" style="background:var(--c-bg,#05050f);">

	<!-- Top bar -->
	<div class="flex items-center gap-3 px-3 py-3 shrink-0" style="border-bottom:1px solid var(--c-border); background:var(--c-surface);">
		<button
			onclick={onclose}
			class="flex items-center justify-center w-8 h-8 rounded-lg transition shrink-0"
			style="color:var(--c-muted); background:transparent;"
			onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.color='var(--c-text)'; (e.currentTarget as HTMLElement).style.background='rgba(255,255,255,.06)'; }}
			onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.color='var(--c-muted)'; (e.currentTarget as HTMLElement).style.background='transparent'; }}
			aria-label={$t('common.close')}
		>
			<svg class="w-5 h-5" viewBox="0 0 20 20" fill="currentColor"><path fill-rule="evenodd" d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z" clip-rule="evenodd"/></svg>
		</button>
		<span class="text-sm font-semibold" style="color:var(--c-text);">{app.name}</span>
		{#if userId}
			<span style="color:var(--c-border);">/</span>
			<span class="text-xs px-2 py-0.5 rounded" style="background:rgba(248,113,113,.12); color:#f87171;">admin view</span>
		{/if}
		<span style="color:var(--c-border);">/</span>
		<span class="text-sm" style="color:var(--c-muted);">{$t('db.database')}</span>
		{#if !loading}
			<span class="text-xs px-2 py-0.5 rounded-full" style="background:rgba(124,124,255,.12); color:#7c7cff;">{$t('db.documents', { n: total })}</span>
		{/if}
		<div class="ml-auto flex items-center gap-2">
			<button
				onclick={() => loadPage(page)}
				disabled={loading}
				class="flex items-center justify-center w-8 h-8 rounded-lg transition shrink-0 disabled:opacity-40"
				style="color:var(--c-muted); border:1px solid var(--c-border); background:transparent;"
				onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.color='var(--c-text)'; (e.currentTarget as HTMLElement).style.background='rgba(255,255,255,.06)'; }}
				onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.color='var(--c-muted)'; (e.currentTarget as HTMLElement).style.background='transparent'; }}
				title={$t('db.reload')}
				aria-label={$t('db.reload')}
			>
				<svg class="w-4 h-4 {loading ? 'animate-spin' : ''}" viewBox="0 0 20 20" fill="currentColor"><path fill-rule="evenodd" d="M4.5 10a5.5 5.5 0 019.401-3.889L15 7.21V4.75a.75.75 0 011.5 0v4.5a.75.75 0 01-.75.75h-4.5a.75.75 0 010-1.5h2.638l-1.03-1.03A4 4 0 106 10a.75.75 0 01-1.5 0z" clip-rule="evenodd"/></svg>
			</button>
			{#if purgeMode && total > 0}
				<button
					onclick={() => openConfirm(
						$t('db.purgeAllDeleted'),
						$t('db.purgeAllConfirm', { n: total }),
						() => { confirmOpen = false; purgeAllDeleted(); }
					)}
					class="text-sm font-semibold px-3 py-1.5 rounded-lg transition"
					style="color:#fff; background:#dc2626;"
					onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.opacity='.85'; }}
					onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.opacity='1'; }}
				>{$t('db.purgeAllDeleted')}</button>
			{:else if total > 0}
				<button
					onclick={() => openConfirm(
						$t('db.clearCollection'),
						$t('db.clearConfirm', { n: total, name: app.name }),
						() => { confirmOpen = false; deleteAll(); }
					)}
					class="text-sm font-medium px-3 py-1.5 rounded-lg transition"
					style="color:#f87171; border:1px solid rgba(248,113,113,.25); background:transparent;"
					onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background='rgba(248,113,113,.08)'; }}
					onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background='transparent'; }}
				>{$t('db.clearCollection')}</button>
			{/if}
		</div>
	</div>

	<!-- Split pane -->
	<div class="flex flex-1 overflow-hidden">

		<!-- ── Left: document list ── -->
		<div class="flex flex-col overflow-hidden shrink-0" style="width:360px; border-right:1px solid var(--c-border);">

			<!-- Search + filter header -->
			<div class="shrink-0 px-3 py-3 flex flex-col gap-2.5" style="border-bottom:1px solid var(--c-border);">
				<div class="relative">
					<svg class="w-4 h-4 absolute left-2.5 top-1/2 -translate-y-1/2 pointer-events-none" style="color:var(--c-muted);" viewBox="0 0 20 20" fill="currentColor"><path fill-rule="evenodd" d="M9 3.5a5.5 5.5 0 100 11 5.5 5.5 0 000-11zM2 9a7 7 0 1112.452 4.391l3.328 3.329a.75.75 0 11-1.06 1.06l-3.329-3.328A7 7 0 012 9z" clip-rule="evenodd"/></svg>
					<input
						type="text"
						bind:value={searchInput}
						oninput={applySearch}
						placeholder={$t('db.searchPlaceholder')}
						class="w-full text-sm rounded-lg pl-8 pr-8 py-2 outline-none"
						style="background:var(--c-bg,#05050f); border:1px solid var(--c-border); color:var(--c-text);"
					/>
					{#if searchInput}
						<button
							onclick={() => { searchInput = ''; search = ''; loadPage(1); }}
							class="absolute right-2 top-1/2 -translate-y-1/2 text-xs"
							style="color:var(--c-muted);"
							aria-label={$t('common.close')}
						>✕</button>
					{/if}
				</div>
				<div class="flex items-center gap-1 text-xs">
					{#each [['active', $t('db.filterActive')], ['deleted', $t('db.filterDeleted')], ['all', $t('db.filterAll')]] as [value, label]}
						<button
							onclick={() => setFilter(value as 'active' | 'deleted' | 'all')}
							class="flex-1 px-2 py-1 rounded-md transition font-medium"
							style="{deletedFilter === value
								? 'background:rgba(124,124,255,.14); color:#7c7cff; border:1px solid rgba(124,124,255,.35);'
								: 'background:transparent; color:var(--c-muted); border:1px solid var(--c-border);'}"
						>{label}</button>
					{/each}
				</div>
			</div>

			<!-- Selection toolbar -->
			{#if !loading && selectableIds.length > 0}
				<div class="shrink-0 flex items-center gap-2 px-3 py-2 text-xs" style="border-bottom:1px solid var(--c-border); background:var(--c-surface);">
					<label class="flex items-center gap-2 cursor-pointer" style="color:var(--c-muted);">
						<input type="checkbox" checked={allSelected} onchange={toggleSelectAll} style="accent-color:#7c7cff;" />
						{selectedIds.length ? $t('db.selectedCount', { n: selectedIds.length }) : $t('db.selectAll')}
					</label>
					{#if selectedIds.length}
						{#if purgeMode}
							<button
								onclick={() => openConfirm(
									$t('db.purgeSelected', { n: selectedIds.length }),
									$t('db.purgeSelectedConfirm', { n: selectedIds.length }),
									() => { confirmOpen = false; purgeSelected(); }
								)}
								class="ml-auto font-semibold px-2 py-1 rounded-md transition"
								style="color:#fff; background:#dc2626;"
								onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.opacity='.85'; }}
								onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.opacity='1'; }}
							>{$t('db.purgeSelected', { n: selectedIds.length })}</button>
						{:else}
							<button
								onclick={() => openConfirm(
									$t('db.deleteDocument'),
									$t('db.deleteSelectedConfirm', { n: selectedIds.length }),
									() => { confirmOpen = false; deleteSelected(); }
								)}
								class="ml-auto font-medium px-2 py-1 rounded-md transition"
								style="color:#f87171; border:1px solid rgba(248,113,113,.3);"
								onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background='rgba(248,113,113,.1)'; }}
								onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background='transparent'; }}
							>{$t('db.deleteSelected', { n: selectedIds.length })}</button>
						{/if}
						<button onclick={clearSelection} class="px-2 py-1 rounded-md" style="color:var(--c-muted); border:1px solid var(--c-border);">{$t('db.deselect')}</button>
					{/if}
				</div>
			{/if}

			<!-- List body -->
			<div class="flex-1 overflow-y-auto">
				{#if loading}
					<div class="flex justify-center py-16">
						<div class="w-5 h-5 rounded-full border-2 animate-spin" style="border-color:#7c7cff; border-top-color:transparent;"></div>
					</div>
				{:else if listError}
					<div class="p-6 text-center">
						<p class="text-sm mb-2" style="color:#f87171;">{listError}</p>
						<button onclick={() => loadPage(page)} class="text-xs" style="color:#7c7cff;">{$t('common.retry')}</button>
					</div>
				{:else if docs.length === 0}
					<div class="p-8 text-center">
						{#if search || deletedFilter !== 'active'}
							<p class="text-sm" style="color:var(--c-muted);">{$t('db.noResults')}</p>
						{:else}
							<p class="text-sm" style="color:var(--c-muted);">{$t('db.noDocuments')}</p>
							<p class="text-xs mt-1 opacity-60" style="color:var(--c-muted);">{$t('db.syncHint')}</p>
						{/if}
					</div>
				{:else}
					{#each docs as doc (doc._id)}
						<div
							role="button"
							tabindex="0"
							onclick={() => selectDoc(doc)}
							onkeydown={(e) => e.key === 'Enter' && selectDoc(doc)}
							class="flex items-center gap-2 px-4 py-3 cursor-pointer"
							style="border-bottom:1px solid var(--c-border); {selectedDoc?._id === doc._id ? 'background:rgba(124,124,255,.08); border-left:2px solid #7c7cff;' : 'border-left:2px solid transparent;'}"
							onmouseenter={(e) => { if (selectedDoc?._id !== doc._id) (e.currentTarget as HTMLElement).style.background='var(--c-surface)'; }}
							onmouseleave={(e) => { if (selectedDoc?._id !== doc._id) (e.currentTarget as HTMLElement).style.background='transparent'; }}
						>
							{#if purgeMode || !isDeleted(doc)}
								<input
									type="checkbox"
									checked={selectedIds.includes(doc._id)}
									onclick={(e) => e.stopPropagation()}
									onchange={() => toggleSelect(doc._id)}
									class="shrink-0"
									style="accent-color:{purgeMode ? '#dc2626' : '#7c7cff'};"
									aria-label={$t('db.selectDoc')}
								/>
							{:else}
								<span class="shrink-0" style="width:13px;"></span>
							{/if}
							<div class="flex-1 min-w-0">
								<div class="flex items-center gap-1.5">
									{#if isDeleted(doc)}
										<span class="shrink-0 text-[10px] px-1 py-0.5 rounded leading-none" style="background:rgba(248,113,113,.14); color:#f87171;">{$t('db.deletedBadge')}</span>
									{/if}
									<p class="text-xs font-mono truncate" style="color:{selectedDoc?._id === doc._id ? '#7c7cff' : 'var(--c-text)'};">{doc._id}</p>
								</div>
								<p class="text-xs truncate mt-0.5" style="color:var(--c-muted);">{preview(doc)}</p>
							</div>
							{#if !isDeleted(doc)}
								<button
									onclick={(e) => { e.stopPropagation(); openConfirm($t('db.deleteDocument'), $t('db.deleteDocConfirm', { id: doc._id }), () => { confirmOpen = false; deleteDoc(doc); }); }}
									class="shrink-0 text-xs px-1.5 py-0.5 rounded opacity-0 transition"
									style="color:#f87171; border:1px solid rgba(248,113,113,.3);"
									onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.opacity='1'; (e.currentTarget as HTMLElement).style.background='rgba(248,113,113,.1)'; }}
									onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.opacity='0'; (e.currentTarget as HTMLElement).style.background='transparent'; }}
								>✕</button>
							{/if}
						</div>
					{/each}
				{/if}
			</div>

			<!-- Pagination -->
			{#if pages > 1}
				<div class="flex items-center justify-between px-4 py-3 shrink-0" style="border-top:1px solid var(--c-border);">
					<button
						onclick={() => loadPage(page - 1)}
						disabled={page <= 1}
						class="text-xs px-2 py-1 rounded disabled:opacity-40 transition"
						style="border:1px solid var(--c-border); color:var(--c-muted);"
					>{$t('db.paginationBack')}</button>
					<span class="text-xs" style="color:var(--c-muted);">{page} / {pages}</span>
					<button
						onclick={() => loadPage(page + 1)}
						disabled={page >= pages}
						class="text-xs px-2 py-1 rounded disabled:opacity-40 transition"
						style="border:1px solid var(--c-border); color:var(--c-muted);"
					>{$t('db.paginationNext')}</button>
				</div>
			{/if}
		</div>

		<!-- ── Right: editor ── -->
		<div class="flex-1 flex flex-col overflow-hidden">
			{#if selectedDoc}
				<!-- Editor toolbar -->
				<div class="flex items-center gap-3 px-5 py-3 shrink-0" style="border-bottom:1px solid var(--c-border); background:var(--c-surface);">
					<span class="text-xs font-mono truncate flex-1" style="color:var(--c-muted);">{selectedDoc._id}</span>
					{#if isDeleted(selectedDoc)}
						<span class="text-xs px-2 py-0.5 rounded" style="background:rgba(248,113,113,.12); color:#f87171;">{$t('db.deletedBadge')}</span>
					{/if}
					{#if !isDeleted(selectedDoc)}
						<button
							onclick={() => openConfirm($t('db.deleteDocument'), $t('db.deleteDocConfirm', { id: selectedDoc._id }), () => { confirmOpen = false; deleteDoc(selectedDoc); })}
							class="text-sm font-medium px-3 py-1 rounded-lg transition"
							style="color:#f87171; border:1px solid rgba(248,113,113,.25);"
							onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background='rgba(248,113,113,.08)'; }}
							onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background='transparent'; }}
						>{$t('db.deleteDocument')}</button>
					{/if}
					{#if !userId}
						<button
							onclick={saveDoc}
							disabled={saving}
							class="text-sm font-semibold px-4 py-1 rounded-lg disabled:opacity-60 transition"
							style="background:#7c7cff; color:#05050f;"
							onmouseenter={(e) => { if (!saving) (e.currentTarget as HTMLElement).style.background='#9090ff'; }}
							onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background='#7c7cff'; }}
						>{saving ? $t('common.saving') : $t('db.save')}</button>
					{/if}
				</div>

				{#if saveError}
					<div class="px-5 pt-3 shrink-0">
						<p class="text-sm px-3 py-2 rounded-lg" style="color:#f87171; background:rgba(248,113,113,.08); border:1px solid rgba(248,113,113,.2);">{saveError}</p>
					</div>
				{/if}

				<!-- Editor fills remaining height -->
				<div class="flex-1 overflow-hidden p-4">
					<div style="height:100%; border:1px solid var(--c-border); border-radius:8px; overflow:hidden;">
						<CodeEditor
							value={editJson}
							onchange={(v) => { editJson = v; saveError = ''; }}
							minHeight="100%"
						/>
					</div>
				</div>

			{:else}
				<!-- Empty state -->
				<div class="flex-1 flex items-center justify-center" style="color:var(--c-muted);">
					<div class="text-center">
						<svg class="w-10 h-10 mx-auto mb-3 opacity-30" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"/></svg>
						<p class="text-sm">{$t('db.selectDocument')}</p>
					</div>
				</div>
			{/if}
		</div>
	</div>
</div>

<ConfirmDialog
	open={confirmOpen}
	title={confirmTitle}
	message={confirmMessage}
	confirmLabel="Delete"
	destructive={true}
	onConfirm={confirmAction}
	onCancel={() => { confirmOpen = false; }}
/>
