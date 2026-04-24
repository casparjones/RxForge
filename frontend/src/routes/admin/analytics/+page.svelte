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
					borderColor: '#ef4444',
					backgroundColor: 'rgba(239, 68, 68, 0.1)',
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
					backgroundColor: 'rgba(239, 68, 68, 0.7)',
					borderColor: '#ef4444',
					borderWidth: 1,
					borderRadius: 6,
				},
			],
		};
	});

	const chartOptions = {
		responsive: true,
		plugins: {
			legend: { display: false },
		},
		scales: {
			y: { beginAtZero: true, grid: { color: '#f3f4f6' } },
			x: { grid: { display: false } },
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
	<h1 class="text-2xl font-bold text-gray-900">Global Analytics</h1>

	{#if loading}
		<div class="flex justify-center py-16">
			<div class="w-8 h-8 border-4 border-red-200 border-t-red-600 rounded-full animate-spin"></div>
		</div>
	{:else if !stats}
		<div class="text-center py-16 bg-white rounded-2xl border border-gray-200">
			<p class="text-gray-500">No analytics data available.</p>
		</div>
	{:else}
		<!-- Stat Cards -->
		<div class="grid grid-cols-1 sm:grid-cols-3 gap-4">
			<div class="bg-white rounded-2xl border border-gray-200 p-6">
				<p class="text-sm text-gray-500">Total Requests</p>
				<p class="text-3xl font-bold text-red-600 mt-1">{stats.total_requests ?? 0}</p>
			</div>
			<div class="bg-white rounded-2xl border border-gray-200 p-6">
				<p class="text-sm text-gray-500">Active Apps</p>
				<p class="text-3xl font-bold text-red-600 mt-1">{stats.active_apps ?? 0}</p>
			</div>
			<div class="bg-white rounded-2xl border border-gray-200 p-6">
				<p class="text-sm text-gray-500">Active Users</p>
				<p class="text-3xl font-bold text-red-600 mt-1">{stats.active_users ?? 0}</p>
			</div>
		</div>

		<!-- Charts Row -->
		<div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
			<!-- Total Requests Line Chart -->
			{#if lineChartData}
				<div class="bg-white rounded-2xl border border-gray-200 p-6">
					<h2 class="text-base font-semibold text-gray-800 mb-4">Total Requests per Day (Last 30 Days)</h2>
					<Line data={lineChartData} options={chartOptions} />
				</div>
			{/if}

			<!-- Top Apps Bar Chart -->
			{#if barChartData}
				<div class="bg-white rounded-2xl border border-gray-200 p-6">
					<h2 class="text-base font-semibold text-gray-800 mb-4">Top Apps by Request Volume</h2>
					<Bar data={barChartData} options={chartOptions} />
				</div>
			{/if}
		</div>
	{/if}
</div>
