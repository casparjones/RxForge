<script lang="ts">
	import { onMount } from 'svelte';
	import { get } from 'svelte/store';
	import { api, type SyncDevice, type SyncEvent } from '$lib/api';
	import { t } from '$lib/i18n';

	interface Props {
		app: any;
		onclose: () => void;
	}

	let { app, onclose }: Props = $props();

	let devices = $state<SyncDevice[]>([]);
	let events = $state<SyncEvent[]>([]);
	let total = $state(0);
	let page = $state(1);
	let pages = $state(1);
	const PER_PAGE = 50;

	let loading = $state(false);
	let error = $state('');

	// Filters
	let deviceFilter = $state('');
	let opFilter = $state('');
	let docFilter = $state('');

	let hasFilter = $derived(!!deviceFilter || !!opFilter || !!docFilter);

	async function load(p = page) {
		loading = true;
		error = '';
		try {
			const [evts, devs] = await Promise.all([
				api.apps.syncEvents.list(app.id, p, PER_PAGE, {
					device_id: deviceFilter || undefined,
					op: opFilter || undefined,
					doc_id: docFilter || undefined,
				}),
				api.apps.syncEvents.devices(app.id),
			]);
			events = evts.events;
			total = evts.total;
			page = evts.page;
			pages = evts.pages;
			devices = devs;
		} catch (e: any) {
			error = e.message;
		} finally {
			loading = false;
		}
	}

	function setDevice(id: string) {
		deviceFilter = deviceFilter === id ? '' : id;
		load(1);
	}

	function setOp(op: string) {
		opFilter = opFilter === op ? '' : op;
		load(1);
	}

	function clearFilters() {
		deviceFilter = '';
		opFilter = '';
		docFilter = '';
		load(1);
	}

	function deviceName(ev: { device_label: string; device_id: string }): string {
		if (ev.device_label) return ev.device_label;
		if (ev.device_id) return ev.device_id.slice(0, 8);
		return get(t)('syncLog.unknownDevice');
	}

	function fmtTime(iso: string): string {
		const d = new Date(iso);
		return d.toLocaleString(undefined, {
			year: '2-digit', month: '2-digit', day: '2-digit',
			hour: '2-digit', minute: '2-digit', second: '2-digit',
		});
	}

	const OP_COLORS: Record<string, string> = {
		write: '#34d399',
		delete: '#f87171',
		conflict: '#fbbf24',
	};

	function opLabel(op: string): string {
		const tr = get(t);
		if (op === 'write') return tr('syncLog.opWrite');
		if (op === 'delete') return tr('syncLog.opDelete');
		return tr('syncLog.opConflict');
	}

	// A device whose deletions dominate its writes is the signature of the
	// "local storage was evicted, so everything looks deleted" failure mode.
	function isSuspicious(d: SyncDevice): boolean {
		return d.deletes >= 10 && d.deletes > d.writes;
	}

	onMount(() => { load(1); });
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
		<span class="text-sm" style="color:var(--c-muted);">{$t('syncLog.title')}</span>
		{#if !loading}
			<span class="text-xs px-2 py-0.5 rounded-full" style="background:rgba(124,124,255,.12); color:#7c7cff;">{$t('syncLog.events', { n: total })}</span>
		{/if}
		<div class="ml-auto flex items-center gap-2">
			{#if hasFilter}
				<button
					onclick={clearFilters}
					class="text-xs font-medium px-3 py-1.5 rounded-lg transition"
					style="color:var(--c-muted); border:1px solid var(--c-border); background:transparent;"
				>{$t('syncLog.clearFilter')}</button>
			{/if}
			<button
				onclick={() => load(page)}
				disabled={loading}
				class="flex items-center justify-center w-8 h-8 rounded-lg transition shrink-0 disabled:opacity-40"
				style="color:var(--c-muted); border:1px solid var(--c-border); background:transparent;"
				title={$t('syncLog.reload')}
				aria-label={$t('syncLog.reload')}
			>
				<svg class="w-4 h-4 {loading ? 'animate-spin' : ''}" viewBox="0 0 20 20" fill="currentColor"><path fill-rule="evenodd" d="M4.5 10a5.5 5.5 0 019.401-3.889L15 7.21V4.75a.75.75 0 011.5 0v4.5a.75.75 0 01-.75.75h-4.5a.75.75 0 010-1.5h2.638l-1.03-1.03A4 4 0 106 10a.75.75 0 01-1.5 0z" clip-rule="evenodd"/></svg>
			</button>
		</div>
	</div>

	<div class="flex flex-1 overflow-hidden">

		<!-- ── Left: devices ── -->
		<div class="flex flex-col overflow-hidden shrink-0" style="width:300px; border-right:1px solid var(--c-border);">
			<div class="shrink-0 px-3 py-3" style="border-bottom:1px solid var(--c-border);">
				<span class="text-xs font-semibold uppercase tracking-wide" style="color:var(--c-muted);">{$t('syncLog.devices')}</span>
			</div>
			<div class="flex-1 overflow-y-auto">
				{#if devices.length === 0}
					<p class="px-3 py-4 text-sm" style="color:var(--c-muted);">{$t('syncLog.noEvents')}</p>
				{:else}
					{#each devices as d (d.device_id)}
						<button
							onclick={() => setDevice(d.device_id)}
							class="w-full text-left px-3 py-2.5 transition"
							style="border-bottom:1px solid var(--c-border); background:{deviceFilter === d.device_id ? 'rgba(124,124,255,.10)' : 'transparent'};"
						>
							<div class="flex items-center gap-2">
								<span class="text-sm font-medium truncate" style="color:var(--c-text);">{deviceName(d)}</span>
								{#if isSuspicious(d)}
									<span title={$t('syncLog.massDeleteWarning')} style="color:#f87171;">⚠</span>
								{/if}
							</div>
							<div class="text-xs mt-0.5 truncate" style="color:var(--c-muted);">
								{d.platform || '—'}{d.app_version ? ' · v' + d.app_version : ''}
							</div>
							<div class="text-xs mt-1 flex gap-2">
								<span style="color:#34d399;">{d.writes}↑</span>
								<span style="color:#f87171;">{d.deletes}✕</span>
								{#if d.conflicts > 0}<span style="color:#fbbf24;">{d.conflicts}⚡</span>{/if}
							</div>
							<div class="text-xs mt-1" style="color:var(--c-muted);">
								{$t('syncLog.lastSeen')}: {fmtTime(d.last_seen)}
							</div>
						</button>
					{/each}
				{/if}
			</div>
		</div>

		<!-- ── Right: event list ── -->
		<div class="flex flex-col flex-1 overflow-hidden">

			<!-- Operation filter -->
			<div class="shrink-0 px-3 py-3 flex items-center gap-2" style="border-bottom:1px solid var(--c-border);">
				{#each ['write', 'delete', 'conflict'] as op}
					<button
						onclick={() => setOp(op)}
						class="text-xs font-medium px-2.5 py-1 rounded-lg transition"
						style="color:{opFilter === op ? OP_COLORS[op] : 'var(--c-muted)'};
						       border:1px solid {opFilter === op ? OP_COLORS[op] : 'var(--c-border)'};
						       background:{opFilter === op ? OP_COLORS[op] + '1a' : 'transparent'};"
					>{opLabel(op)}</button>
				{/each}
				{#if docFilter}
					<span class="text-xs ml-2 px-2 py-1 rounded" style="color:var(--c-muted); background:var(--c-surface-2);">
						{$t('syncLog.docFilterHint', { id: docFilter })}
					</span>
				{/if}
			</div>

			<div class="flex-1 overflow-y-auto">
				{#if error}
					<p class="px-3 py-4 text-sm" style="color:#f87171;">{error}</p>
				{:else if loading && events.length === 0}
					<p class="px-3 py-4 text-sm" style="color:var(--c-muted);">…</p>
				{:else if events.length === 0}
					<div class="px-3 py-6">
						<p class="text-sm" style="color:var(--c-muted);">{hasFilter ? $t('syncLog.noResults') : $t('syncLog.noEvents')}</p>
						{#if !hasFilter}
							<p class="text-xs mt-1" style="color:var(--c-muted);">{$t('syncLog.noEventsHint')}</p>
						{/if}
					</div>
				{:else}
					<table class="w-full text-sm" style="border-collapse:collapse;">
						<thead>
							<tr style="border-bottom:1px solid var(--c-border);">
								<th class="text-left px-3 py-2 text-xs font-semibold uppercase tracking-wide" style="color:var(--c-muted);">{$t('syncLog.colTime')}</th>
								<th class="text-left px-3 py-2 text-xs font-semibold uppercase tracking-wide" style="color:var(--c-muted);">{$t('syncLog.colOp')}</th>
								<th class="text-left px-3 py-2 text-xs font-semibold uppercase tracking-wide" style="color:var(--c-muted);">{$t('syncLog.colDoc')}</th>
								<th class="text-left px-3 py-2 text-xs font-semibold uppercase tracking-wide" style="color:var(--c-muted);">{$t('syncLog.colDevice')}</th>
								<th class="text-left px-3 py-2 text-xs font-semibold uppercase tracking-wide" style="color:var(--c-muted);">{$t('syncLog.colReason')}</th>
							</tr>
						</thead>
						<tbody>
							{#each events as ev (ev.id)}
								<tr style="border-bottom:1px solid var(--c-border);">
									<td class="px-3 py-2 whitespace-nowrap font-mono text-xs" style="color:var(--c-muted);">{fmtTime(ev.created_at)}</td>
									<td class="px-3 py-2 whitespace-nowrap">
										<span class="text-xs px-2 py-0.5 rounded-full" style="color:{OP_COLORS[ev.op]}; background:{OP_COLORS[ev.op]}1a;">{opLabel(ev.op)}</span>
									</td>
									<td class="px-3 py-2 font-mono text-xs break-all">
										<button
											onclick={() => { docFilter = docFilter === ev.doc_id ? '' : ev.doc_id; load(1); }}
											style="color:var(--c-text); background:transparent; text-align:left;"
											title={ev.doc_id}
										>{ev.doc_id}</button>
									</td>
									<td class="px-3 py-2 whitespace-nowrap">
										<button
											onclick={() => setDevice(ev.device_id)}
											style="color:var(--c-text); background:transparent;"
											title={ev.device_id || '—'}
										>{deviceName(ev)}</button>
									</td>
									<td class="px-3 py-2 whitespace-nowrap text-xs" style="color:var(--c-muted);">{ev.reason || '—'}</td>
								</tr>
							{/each}
						</tbody>
					</table>
				{/if}
			</div>

			<!-- Pagination -->
			{#if pages > 1}
				<div class="shrink-0 flex items-center justify-between px-3 py-2" style="border-top:1px solid var(--c-border);">
					<button
						onclick={() => load(page - 1)}
						disabled={page <= 1 || loading}
						class="text-sm px-3 py-1.5 rounded-lg transition disabled:opacity-40"
						style="color:var(--c-muted); border:1px solid var(--c-border); background:transparent;"
					>{$t('db.paginationBack')}</button>
					<span class="text-xs" style="color:var(--c-muted);">{page} / {pages}</span>
					<button
						onclick={() => load(page + 1)}
						disabled={page >= pages || loading}
						class="text-sm px-3 py-1.5 rounded-lg transition disabled:opacity-40"
						style="color:var(--c-muted); border:1px solid var(--c-border); background:transparent;"
					>{$t('db.paginationNext')}</button>
				</div>
			{/if}
		</div>
	</div>
</div>
