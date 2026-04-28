<script lang="ts">
	import { api } from '$lib/api';
	import { toast } from '$lib/stores/toast';
	import { get } from 'svelte/store';
	import { t } from '$lib/i18n';
	import CodeEditor from './CodeEditor.svelte';
	import ConfirmDialog from './ConfirmDialog.svelte';

	interface Props {
		app: any;
		onclose: () => void;
	}

	let { app, onclose }: Props = $props();

	// List state
	let docs = $state<any[]>([]);
	let total = $state(0);
	let page = $state(1);
	let pages = $state(1);
	const PER_PAGE = 20;
	let loading = $state(false);
	let listError = $state('');

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
		loading = true; listError = '';
		try {
			const res = await api.apps.db.list(app.id, p, PER_PAGE);
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
		if (!selectedDoc) return;
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
			await api.apps.db.deleteDoc(app.id, doc._id, doc._rev);
			docs = docs.filter(d => d._id !== doc._id);
			total = Math.max(0, total - 1);
			if (selectedDoc?._id === doc._id) selectedDoc = null;
			toast.success(get(t)('db.documentDeleted'));
		} catch (e: any) {
			toast.error('Error: ' + e.message);
		}
	}

	async function deleteAll() {
		try {
			const res = await api.apps.db.deleteAll(app.id);
			docs = []; total = 0; page = 1; pages = 1; selectedDoc = null;
			toast.success(`${res.deleted} document(s) deleted.`);
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
		<span style="color:var(--c-border);">/</span>
		<span class="text-sm" style="color:var(--c-muted);">{$t('db.database')}</span>
		{#if !loading}
			<span class="text-xs px-2 py-0.5 rounded-full" style="background:rgba(124,124,255,.12); color:#7c7cff;">{$t('db.documents', { n: total })}</span>
		{/if}
		<div class="ml-auto flex items-center gap-2">
			{#if docs.length > 0}
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
						<p class="text-sm" style="color:var(--c-muted);">{$t('db.noDocuments')}</p>
						<p class="text-xs mt-1 opacity-60" style="color:var(--c-muted);">{$t('db.syncHint')}</p>
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
							<div class="flex-1 min-w-0">
								<p class="text-xs font-mono truncate" style="color:{selectedDoc?._id === doc._id ? '#7c7cff' : 'var(--c-text)'};">{doc._id}</p>
								<p class="text-xs truncate mt-0.5" style="color:var(--c-muted);">{preview(doc)}</p>
							</div>
							<button
								onclick={(e) => { e.stopPropagation(); openConfirm($t('db.deleteDocument'), $t('db.deleteDocConfirm', { id: doc._id }), () => { confirmOpen = false; deleteDoc(doc); }); }}
								class="shrink-0 text-xs px-1.5 py-0.5 rounded opacity-0 transition"
								style="color:#f87171; border:1px solid rgba(248,113,113,.3);"
								onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.opacity='1'; (e.currentTarget as HTMLElement).style.background='rgba(248,113,113,.1)'; }}
								onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.opacity='0'; (e.currentTarget as HTMLElement).style.background='transparent'; }}
							>✕</button>
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
					<button
						onclick={() => openConfirm($t('db.deleteDocument'), $t('db.deleteDocConfirm', { id: selectedDoc._id }), () => { confirmOpen = false; deleteDoc(selectedDoc); })}
						class="text-sm font-medium px-3 py-1 rounded-lg transition"
						style="color:#f87171; border:1px solid rgba(248,113,113,.25);"
						onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background='rgba(248,113,113,.08)'; }}
						onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background='transparent'; }}
					>{$t('db.deleteDocument')}</button>
					<button
						onclick={saveDoc}
						disabled={saving}
						class="text-sm font-semibold px-4 py-1 rounded-lg disabled:opacity-60 transition"
						style="background:#7c7cff; color:#05050f;"
						onmouseenter={(e) => { if (!saving) (e.currentTarget as HTMLElement).style.background='#9090ff'; }}
						onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background='#7c7cff'; }}
					>{saving ? $t('common.saving') : $t('db.save')}</button>
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
