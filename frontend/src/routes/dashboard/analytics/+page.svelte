<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import { toast } from '$lib/stores/toast';
	import { t } from '$lib/i18n';
	import { Line } from 'svelte-chartjs';
	import {
		Chart as ChartJS,
		CategoryScale, LinearScale, PointElement, LineElement,
		Title, Tooltip, Legend, Filler,
	} from 'chart.js';

	ChartJS.register(CategoryScale, LinearScale, PointElement, LineElement, Title, Tooltip, Legend, Filler);

	let apps = $state<any[]>([]);
	let selectedAppId = $state<string | null>(null);
	let stats = $state<any>(null);
	let loading = $state(true);

	const chartData = $derived.by(() => {
		if (!stats?.daily_requests) return null;
		return {
			labels: stats.daily_requests.map((d: any) => d.date),
			datasets: [{
				label: $t('analytics.requestsPerDay'),
				data: stats.daily_requests.map((d: any) => d.count),
				borderColor: '#7c7cff',
				backgroundColor: 'rgba(124,124,255,0.08)',
				fill: true, tension: 0.4,
				pointBackgroundColor: '#7c7cff',
				pointRadius: 3,
			}],
		};
	});

	const chartOptions = {
		responsive: true,
		plugins: { legend: { display: false } },
		scales: {
			y: { beginAtZero: true, grid: { color: 'rgba(124,124,255,0.07)' }, ticks: { color: '#8b8fa8', font: { family: 'JetBrains Mono', size: 11 } } },
			x: { grid: { display: false }, ticks: { color: '#8b8fa8', font: { family: 'JetBrains Mono', size: 11 } } },
		},
	};

	async function loadApps() {
		try {
			apps = await api.apps.list();
			if (apps.length > 0) { selectedAppId = apps[0].id; await loadStats(); }
		} catch (e: any) { toast.error('Failed to load apps: ' + e.message); }
		finally { loading = false; }
	}

	async function loadStats() {
		if (!selectedAppId) return;
		loading = true;
		try { stats = await api.apps.getStats(selectedAppId); }
		catch (e: any) { toast.error('Failed to load stats: ' + e.message); stats = null; }
		finally { loading = false; }
	}

	onMount(loadApps);
</script>

<div class="space-y-6">
	<div class="flex items-center justify-between flex-wrap gap-3">
		<div>
			<div style="font-family:'JetBrains Mono',monospace; font-size:10px; letter-spacing:.15em; color:#7c7cff; margin-bottom:8px; text-transform:uppercase;">── {$t('analytics.sectionLabel')}</div>
			<h1 class="text-2xl font-semibold" style="letter-spacing:-.02em;">{$t('analytics.title')}</h1>
		</div>
		{#if apps.length > 0}
			<select
				bind:value={selectedAppId}
				onchange={loadStats}
				class="px-4 py-2 rounded-lg text-sm outline-none"
				style="background:var(--c-surface); border:1px solid var(--c-border); color:var(--c-text); font-family:'JetBrains Mono',monospace;"
			>
				{#each apps as app (app.id)}
					<option value={app.id}>{app.name}</option>
				{/each}
			</select>
		{/if}
	</div>

	{#if loading}
		<div class="flex justify-center py-16">
			<div class="w-6 h-6 rounded-full border-2 border-t-transparent animate-spin" style="border-color:#7c7cff; border-top-color:transparent;"></div>
		</div>
		<p style="color:var(--c-muted); font-family:'JetBrains Mono',monospace; font-size:12px; text-align:center;">{$t('common.loading')}</p>
	{:else if !stats}
		<div class="text-center py-16 rounded-xl" style="background:var(--c-surface); border:1px solid var(--c-border);">
			<p style="color:var(--c-muted); font-family:'JetBrains Mono',monospace; font-size:12px;">{$t('analytics.noData')}</p>
		</div>
	{:else}
		<div class="grid grid-cols-1 sm:grid-cols-3 gap-4">
			{#each [
				{ label: 'Requests Today', value: stats.requests_today ?? 0 },
				{ label: 'Last 7 Days',    value: stats.requests_7d ?? 0 },
				{ label: 'Last 30 Days',   value: stats.requests_30d ?? 0 },
			] as card}
				<div class="rounded-xl p-6" style="background:var(--c-surface); border:1px solid var(--c-border);">
					<p style="font-family:'JetBrains Mono',monospace; font-size:9px; letter-spacing:.12em; text-transform:uppercase; color:var(--c-muted); margin-bottom:8px;">{card.label}</p>
					<p class="text-3xl font-bold" style="color:#7c7cff;">{card.value}</p>
				</div>
			{/each}
		</div>

		{#if chartData}
			<div class="rounded-xl p-6" style="background:var(--c-surface); border:1px solid var(--c-border);">
				<p style="font-family:'JetBrains Mono',monospace; font-size:9px; letter-spacing:.12em; text-transform:uppercase; color:var(--c-muted); margin-bottom:20px;">Requests · Last 30 Days</p>
				<Line data={chartData} options={chartOptions} />
			</div>
		{/if}
	{/if}

	{#if apps.length === 0 && !loading}
		<div class="text-center py-16 rounded-xl" style="background:var(--c-surface); border:1px solid var(--c-border);">
			<p style="color:var(--c-muted); font-size:14px;">{$t('analytics.noData')} <a href="/dashboard/apps" style="color:#7c7cff;">{$t('dashboard.newApp')}</a></p>
		</div>
	{/if}
</div>
