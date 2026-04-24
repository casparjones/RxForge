<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import { auth } from '$lib/stores/auth';
	import { toast } from '$lib/stores/toast';
	import { Line } from 'svelte-chartjs';
	import {
		Chart as ChartJS,
		CategoryScale,
		LinearScale,
		PointElement,
		LineElement,
		Title,
		Tooltip,
		Legend,
		Filler,
	} from 'chart.js';

	ChartJS.register(CategoryScale, LinearScale, PointElement, LineElement, Title, Tooltip, Legend, Filler);

	let apps = $state<any[]>([]);
	let selectedAppId = $state<string | null>(null);
	let stats = $state<any>(null);
	let loading = $state(true);

	const chartData = $derived.by(() => {
		if (!stats?.daily_requests) return null;
		const labels = stats.daily_requests.map((d: any) => d.date);
		const data = stats.daily_requests.map((d: any) => d.count);
		return {
			labels,
			datasets: [
				{
					label: 'Requests per Day',
					data,
					borderColor: '#6366f1',
					backgroundColor: 'rgba(99, 102, 241, 0.1)',
					fill: true,
					tension: 0.4,
				},
			],
		};
	});

	const chartOptions = {
		responsive: true,
		plugins: {
			legend: { display: false },
			title: { display: false },
		},
		scales: {
			y: { beginAtZero: true, grid: { color: '#f3f4f6' } },
			x: { grid: { display: false } },
		},
	};

	async function loadApps() {
		try {
			apps = await api.apps.list();
			if (apps.length > 0) {
				selectedAppId = apps[0].id;
				await loadStats();
			}
		} catch (e: any) {
			toast.error('Failed to load apps: ' + e.message);
		} finally {
			loading = false;
		}
	}

	async function loadStats() {
		if (!selectedAppId) return;
		loading = true;
		try {
			stats = await api.apps.getStats(selectedAppId);
		} catch (e: any) {
			toast.error('Failed to load stats: ' + e.message);
			stats = null;
		} finally {
			loading = false;
		}
	}

	onMount(loadApps);
</script>

<div class="space-y-6">
	<div class="flex items-center justify-between flex-wrap gap-3">
		<h1 class="text-2xl font-bold text-gray-900">App Analytics</h1>
		{#if apps.length > 0}
			<select
				bind:value={selectedAppId}
				onchange={loadStats}
				class="px-4 py-2 border border-gray-300 rounded-lg text-sm bg-white focus:ring-2 focus:ring-indigo-500 outline-none"
			>
				{#each apps as app (app.id)}
					<option value={app.id}>{app.name}</option>
				{/each}
			</select>
		{/if}
	</div>

	{#if loading}
		<div class="flex justify-center py-16">
			<div class="w-8 h-8 border-4 border-indigo-200 border-t-indigo-600 rounded-full animate-spin"></div>
		</div>
	{:else if !stats}
		<div class="text-center py-16 bg-white rounded-2xl border border-gray-200">
			<p class="text-gray-500">No analytics data available.</p>
		</div>
	{:else}
		<!-- Stat Cards -->
		<div class="grid grid-cols-1 sm:grid-cols-3 gap-4">
			<div class="bg-white rounded-2xl border border-gray-200 p-6">
				<p class="text-sm text-gray-500">Requests Today</p>
				<p class="text-3xl font-bold text-indigo-600 mt-1">{stats.requests_today ?? 0}</p>
			</div>
			<div class="bg-white rounded-2xl border border-gray-200 p-6">
				<p class="text-sm text-gray-500">Last 7 Days</p>
				<p class="text-3xl font-bold text-indigo-600 mt-1">{stats.requests_7d ?? 0}</p>
			</div>
			<div class="bg-white rounded-2xl border border-gray-200 p-6">
				<p class="text-sm text-gray-500">Last 30 Days</p>
				<p class="text-3xl font-bold text-indigo-600 mt-1">{stats.requests_30d ?? 0}</p>
			</div>
		</div>

		<!-- Line Chart -->
		{#if chartData}
			<div class="bg-white rounded-2xl border border-gray-200 p-6">
				<h2 class="text-base font-semibold text-gray-800 mb-4">Requests per Day (Last 30 Days)</h2>
				<Line data={chartData} options={chartOptions} />
			</div>
		{/if}
	{/if}

	{#if apps.length === 0 && !loading}
		<div class="text-center py-16 bg-white rounded-2xl border border-gray-200">
			<p class="text-gray-500">No apps found. <a href="/dashboard/apps" class="text-indigo-600 hover:underline">Create an app</a> to see analytics.</p>
		</div>
	{/if}
</div>
