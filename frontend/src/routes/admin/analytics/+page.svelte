<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import { toast } from '$lib/stores/toast';
	import { Line, Bar } from 'svelte-chartjs';
	import {
		Chart as ChartJS,
		CategoryScale,
		LinearScale,
		PointElement,
		LineElement,
		BarElement,
		Title,
		Tooltip,
		Legend,
		Filler,
	} from 'chart.js';

	ChartJS.register(CategoryScale, LinearScale, PointElement, LineElement, BarElement, Title, Tooltip, Legend, Filler);

	let stats = $state<any>(null);
	let loading = $state(true);

	const lineChartData = $derived.by(() => {
		if (!stats?.daily_requests) return null;
		return {
			labels: stats.daily_requests.map((d: any) => d.date),
			datasets: [
				{
					label: 'Total Requests per Day',
					data: stats.daily_requests.map((d: any) => d.count),
					borderColor: '#f87171',
					backgroundColor: 'rgba(248, 113, 113, 0.1)',
					fill: true,
					tension: 0.4,
				},
			],
		};
	});

	const barChartData = $derived.by(() => {
		if (!stats?.top_apps) return null;
		return {
			labels: stats.top_apps.map((a: any) => a.name),
			datasets: [
				{
					label: 'Requests',
					data: stats.top_apps.map((a: any) => a.requests),
					backgroundColor: 'rgba(248, 113, 113, 0.7)',
					borderColor: '#f87171',
					borderWidth: 1,
					borderRadius: 6,
				},
			],
		};
	});

	const chartOptions = {
		responsive: true,
		plugins: { legend: { display: false } },
		scales: {
			y: { beginAtZero: true, grid: { color: 'rgba(139,143,168,0.12)' }, ticks: { color: '#8b8fa8' } },
			x: { grid: { display: false }, ticks: { color: '#8b8fa8' } },
		},
	};

	async function loadStats() {
		loading = true;
		try {
			stats = await api.admin.analytics.global();
		} catch (e: any) {
			toast.error('Failed to load analytics: ' + e.message);
		} finally {
			loading = false;
		}
	}

	onMount(loadStats);
</script>

<div class="space-y-6">
	<h1 class="text-2xl font-bold" style="color:var(--c-text);">Global Analytics</h1>

	{#if loading}
		<div class="flex justify-center py-16">
			<div class="w-8 h-8 border-4 rounded-full animate-spin" style="border-color:rgba(248,113,113,.25); border-top-color:#f87171;"></div>
		</div>
	{:else if !stats}
		<div class="text-center py-16 rounded-2xl" style="background:var(--c-surface); border:1px solid var(--c-border);">
			<p style="color:var(--c-muted);">No analytics data available.</p>
		</div>
	{:else}
		<!-- Stat Cards -->
		<div class="grid grid-cols-1 sm:grid-cols-3 gap-4">
			<div class="rounded-2xl p-6" style="background:var(--c-surface); border:1px solid var(--c-border);">
				<p class="text-sm mb-1" style="color:var(--c-muted);">Total Requests</p>
				<p class="text-3xl font-bold" style="color:#f87171;">{stats.total_requests ?? 0}</p>
			</div>
			<div class="rounded-2xl p-6" style="background:var(--c-surface); border:1px solid var(--c-border);">
				<p class="text-sm mb-1" style="color:var(--c-muted);">Total Apps</p>
				<p class="text-3xl font-bold" style="color:#f87171;">{stats.total_apps ?? 0}</p>
			</div>
			<div class="rounded-2xl p-6" style="background:var(--c-surface); border:1px solid var(--c-border);">
				<p class="text-sm mb-1" style="color:var(--c-muted);">Total Users</p>
				<p class="text-3xl font-bold" style="color:#f87171;">{stats.total_users ?? 0}</p>
			</div>
		</div>

		<!-- Charts Row -->
		<div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
			{#if lineChartData}
				<div class="rounded-2xl p-6" style="background:var(--c-surface); border:1px solid var(--c-border);">
					<h2 class="text-base font-semibold mb-4" style="color:var(--c-text);">Total Requests per Day (Last 30 Days)</h2>
					<Line data={lineChartData} options={chartOptions} />
				</div>
			{/if}

			{#if barChartData}
				<div class="rounded-2xl p-6" style="background:var(--c-surface); border:1px solid var(--c-border);">
					<h2 class="text-base font-semibold mb-4" style="color:var(--c-text);">Top Apps by Request Volume</h2>
					<Bar data={barChartData} options={chartOptions} />
				</div>
			{/if}
		</div>
	{/if}
</div>
