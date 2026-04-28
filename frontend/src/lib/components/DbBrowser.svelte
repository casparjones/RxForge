<script lang="ts">
	import { api } from '$lib/api';
	import { toast } from '$lib/stores/toast';
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

	// Document detail state
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
			docs = res.docs;
			total = res.total;
			page = res.page;
			pages = res.pages;
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

	function backToList() {
		selectedDoc = null;
		saveError = '';
	}

	async function saveDoc() {
		if (!selectedDoc) return;
		saving = true; saveError = '';
		try {
			let parsed: any;
			try { parsed = JSON.parse(editJson); } catch { saveError = 'Ungültiges JSON.'; saving = false; return; }
			const res = await api.apps.db.updateDoc(app.id, selectedDoc._id, parsed);
			// Update local list
			const updated = { ...parsed, _rev: res.rev ?? parsed._rev };
			selectedDoc = updated;
			editJson = JSON.stringify(updated, null, 2);
			docs = docs.map(d => d._id === updated._id ? updated : d);
			toast.success('Dokument gespeichert.');
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
			if (selectedDoc?._id === doc._id) backToList();
			toast.success('Dokument gelöscht.');
		} catch (e: any) {
			toast.error('Fehler: ' + e.message);
		}
	}

	async function deleteAll() {
		try {
			const res = await api.apps.db.deleteAll(app.id);
			docs = []; total = 0; page = 1; pages = 1;
			if (selectedDoc) backToList();
			toast.success(`${res.deleted} Dokument(e) gelöscht.`);
		} catch (e: any) {
			toast.error('Fehler: ' + e.message);
		}
	}

	function preview(doc: any): string {
		const skip = new Set(['_id', '_rev', '_deleted']);
		const entries = Object.entries(doc).filter(([k]) => !skip.has(k)).slice(0, 3);
		if (!entries.length) return '—';
		return entries.map(([k, v]) => {
			const val = typeof v === 'object' ? JSON.stringify(v) : String(v);
			return `${k}: ${val.length > 30 ? val.slice(0, 30) + '…' : val}`;
		}).join('  ·  ');
	}

	// Load first page on mount
	$effect(() => { loadPage(1); });
</script>

<!-- Full-screen overlay -->
<div class="fixed inset-0 z-50 flex flex-col" style="background:var(--c-bg, #05050f);">

	<!-- Top bar -->
	<div class="flex items-center gap-3 px-6 py-4 shrink-0" style="border-bottom:1px solid var(--c-border); background:var(--c-surface);">
		{#if selectedDoc}
			<button
				onclick={backToList}
				class="flex items-center gap-1.5 text-sm transition"
				style="color:var(--c-muted);"
				onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.color='var(--c-text)'; }}
				onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.color='var(--c-muted)'; }}
			>
				← Zurück
			</button>
			<span style="color:var(--c-border);">|</span>
			<span class="text-sm font-mono truncate" style="color:var(--c-muted); max-width:300px;">{selectedDoc._id}</span>
		{:else}
			<span class="text-sm font-semibold" style="color:var(--c-text);">
				{app.name}
				<span class="font-normal" style="color:var(--c-muted);"> — Datenbank</span>
			</span>
			{#if !loading}
				<span class="text-xs px-2 py-0.5 rounded-full" style="background:rgba(124,124,255,.12); color:#7c7cff;">{total} Dokumente</span>
			{/if}
		{/if}
		<div class="ml-auto flex items-center gap-2">
			{#if !selectedDoc && docs.length > 0}
				<button
					onclick={() => openConfirm(
						'Alle Dokumente löschen',
						`Alle ${total} Dokumente in "${app.name}" unwiderruflich löschen?`,
						() => { confirmOpen = false; deleteAll(); }
					)}
					class="text-sm font-medium px-3 py-1.5 rounded-lg transition"
					style="color:#f87171; border:1px solid rgba(248,113,113,.25); background:transparent;"
					onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background='rgba(248,113,113,.08)'; }}
					onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background='transparent'; }}
				>Collection leeren</button>
			{/if}
			{#if selectedDoc}
				<button
					onclick={() => openConfirm(
						'Dokument löschen',
						`"${selectedDoc._id}" löschen?`,
						() => { confirmOpen = false; deleteDoc(selectedDoc); }
					)}
					class="text-sm font-medium px-3 py-1.5 rounded-lg transition"
					style="color:#f87171; border:1px solid rgba(248,113,113,.25); background:transparent;"
					onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background='rgba(248,113,113,.08)'; }}
					onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background='transparent'; }}
				>Dokument löschen</button>
				<button
					onclick={saveDoc}
					disabled={saving}
					class="text-sm font-semibold px-4 py-1.5 rounded-lg disabled:opacity-60 transition"
					style="background:#7c7cff; color:#05050f;"
					onmouseenter={(e) => { if (!saving) (e.currentTarget as HTMLElement).style.background='#9090ff'; }}
					onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background='#7c7cff'; }}
				>{saving ? 'Speichern…' : 'Speichern'}</button>
			{/if}
			<button
				onclick={onclose}
				class="text-sm font-medium px-3 py-1.5 rounded-lg transition"
				style="color:var(--c-muted); border:1px solid var(--c-border); background:transparent;"
				onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.color='var(--c-text)'; }}
				onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.color='var(--c-muted)'; }}
			>Schließen</button>
		</div>
	</div>

	<!-- Content -->
	<div class="flex-1 overflow-auto p-6">

		{#if selectedDoc}
			<!-- ── Document editor ── -->
			<div style="max-width:860px; margin:0 auto;">
				{#if saveError}
					<p class="mb-3 text-sm px-4 py-2 rounded-lg" style="color:#f87171; background:rgba(248,113,113,.08); border:1px solid rgba(248,113,113,.2);">{saveError}</p>
				{/if}
				<CodeEditor
					value={editJson}
					onchange={(v) => { editJson = v; }}
					minHeight="calc(100vh - 200px)"
				/>
			</div>

		{:else}
			<!-- ── Document list ── -->
			{#if loading}
				<div class="flex justify-center py-20">
					<div class="w-6 h-6 rounded-full border-2 animate-spin" style="border-color:#7c7cff; border-top-color:transparent;"></div>
				</div>
			{:else if listError}
				<div class="text-center py-16">
					<p style="color:#f87171;">{listError}</p>
					<button onclick={() => loadPage(page)} class="mt-3 text-sm" style="color:#7c7cff;">Erneut versuchen</button>
				</div>
			{:else if docs.length === 0}
				<div class="text-center py-20">
					<p style="color:var(--c-muted);">Keine Dokumente vorhanden.</p>
					<p class="text-sm mt-1" style="color:var(--c-muted); opacity:.6;">Daten erscheinen hier sobald Sync-Daten eintreffen.</p>
				</div>
			{:else}
				<div style="max-width:1100px; margin:0 auto;">
					<table style="width:100%; border-collapse:collapse; font-size:13px;">
						<thead>
							<tr style="border-bottom:1px solid var(--c-border);">
								<th class="text-left py-2 px-3 text-xs font-semibold uppercase tracking-wide" style="color:var(--c-muted); width:35%;">ID</th>
								<th class="text-left py-2 px-3 text-xs font-semibold uppercase tracking-wide" style="color:var(--c-muted);">Vorschau</th>
								<th class="py-2 px-3" style="width:80px;"></th>
							</tr>
						</thead>
						<tbody>
							{#each docs as doc (doc._id)}
								<tr
									style="border-bottom:1px solid var(--c-border); cursor:pointer; transition:background .12s;"
									onclick={() => selectDoc(doc)}
									onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background='var(--c-surface)'; }}
									onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background='transparent'; }}
								>
									<td class="py-3 px-3 font-mono" style="color:#7c7cff; max-width:0; overflow:hidden; text-overflow:ellipsis; white-space:nowrap;">
										{doc._id}
									</td>
									<td class="py-3 px-3" style="color:var(--c-muted); max-width:0; overflow:hidden; text-overflow:ellipsis; white-space:nowrap;">
										{preview(doc)}
									</td>
									<td class="py-3 px-3 text-right" onclick={(e) => e.stopPropagation()}>
										<button
											onclick={() => openConfirm(
												'Dokument löschen',
												`"${doc._id}" löschen?`,
												() => { confirmOpen = false; deleteDoc(doc); }
											)}
											class="text-xs px-2 py-1 rounded transition"
											style="color:#f87171; border:1px solid rgba(248,113,113,.2);"
											onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background='rgba(248,113,113,.1)'; }}
											onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background='transparent'; }}
										>Löschen</button>
									</td>
								</tr>
							{/each}
						</tbody>
					</table>

					<!-- Pagination -->
					{#if pages > 1}
						<div class="flex items-center justify-center gap-2 mt-6">
							<button
								onclick={() => loadPage(page - 1)}
								disabled={page <= 1}
								class="px-3 py-1.5 text-sm rounded-lg disabled:opacity-40 transition"
								style="border:1px solid var(--c-border); color:var(--c-muted);"
								onmouseenter={(e) => { if (page > 1) (e.currentTarget as HTMLElement).style.color='var(--c-text)'; }}
								onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.color='var(--c-muted)'; }}
							>← Zurück</button>
							<span class="text-sm" style="color:var(--c-muted);">Seite {page} / {pages}</span>
							<button
								onclick={() => loadPage(page + 1)}
								disabled={page >= pages}
								class="px-3 py-1.5 text-sm rounded-lg disabled:opacity-40 transition"
								style="border:1px solid var(--c-border); color:var(--c-muted);"
								onmouseenter={(e) => { if (page < pages) (e.currentTarget as HTMLElement).style.color='var(--c-text)'; }}
								onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.color='var(--c-muted)'; }}
							>Weiter →</button>
						</div>
					{/if}
				</div>
			{/if}
		{/if}
	</div>
</div>

<ConfirmDialog
	open={confirmOpen}
	title={confirmTitle}
	message={confirmMessage}
	confirmLabel="Löschen"
	destructive={true}
	onConfirm={confirmAction}
	onCancel={() => { confirmOpen = false; }}
/>
