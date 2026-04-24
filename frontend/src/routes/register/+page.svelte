<script lang="ts">
	import { goto } from '$app/navigation';
	import { auth } from '$lib/stores/auth';
	import { api } from '$lib/api';
	import { toast } from '$lib/stores/toast';

	let email = $state('');
	let password = $state('');
	let passwordConfirm = $state('');
	let loading = $state(false);
	let error = $state('');

	function validateEmail(e: string) {
		return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(e);
	}

	async function handleRegister() {
		error = '';
		if (!email || !password || !passwordConfirm) {
			error = 'All fields are required.';
			return;
		}
		if (!validateEmail(email)) {
			error = 'Please enter a valid email address.';
			return;
		}
		if (password.length < 8) {
			error = 'Password must be at least 8 characters.';
			return;
		}
		if (password !== passwordConfirm) {
			error = 'Passwords do not match.';
			return;
		}
		loading = true;
		try {
			const res = await api.auth.register(email, password);
			auth.login(res.token, res.user);
			toast.success('Account created! Welcome to RxForge.');
			goto('/dashboard');
		} catch (e: any) {
			error = e.message || 'Registration failed.';
			toast.error(error);
		} finally {
			loading = false;
		}
	}
</script>

<div class="min-h-screen bg-gray-50 flex items-center justify-center py-12 px-4">
	<div class="max-w-md w-full">
		<div class="text-center mb-8">
			<h1 class="text-3xl font-bold text-indigo-600">RxForge</h1>
			<p class="mt-2 text-gray-600">Create your account</p>
		</div>

		<div class="bg-white rounded-2xl shadow-sm border border-gray-200 p-8">
			<form onsubmit={(e) => { e.preventDefault(); handleRegister(); }} class="space-y-5">
				{#if error}
					<div class="bg-red-50 text-red-700 rounded-lg px-4 py-3 text-sm">{error}</div>
				{/if}

				<div>
					<label for="email" class="block text-sm font-medium text-gray-700 mb-1">Email</label>
					<input
						id="email"
						type="email"
						bind:value={email}
						required
						class="w-full px-4 py-2.5 border border-gray-300 rounded-lg focus:ring-2 focus:ring-indigo-500 focus:border-indigo-500 outline-none transition"
						placeholder="you@example.com"
					/>
				</div>

				<div>
					<label for="password" class="block text-sm font-medium text-gray-700 mb-1">Password</label>
					<input
						id="password"
						type="password"
						bind:value={password}
						required
						class="w-full px-4 py-2.5 border border-gray-300 rounded-lg focus:ring-2 focus:ring-indigo-500 focus:border-indigo-500 outline-none transition"
						placeholder="Min. 8 characters"
					/>
				</div>

				<div>
					<label for="passwordConfirm" class="block text-sm font-medium text-gray-700 mb-1">Confirm Password</label>
					<input
						id="passwordConfirm"
						type="password"
						bind:value={passwordConfirm}
						required
						class="w-full px-4 py-2.5 border border-gray-300 rounded-lg focus:ring-2 focus:ring-indigo-500 focus:border-indigo-500 outline-none transition"
						placeholder="••••••••"
					/>
					{#if passwordConfirm && password !== passwordConfirm}
						<p class="text-xs text-red-500 mt-1">Passwords do not match</p>
					{/if}
				</div>

				<button
					type="submit"
					disabled={loading}
					class="w-full bg-indigo-600 hover:bg-indigo-700 disabled:opacity-60 text-white font-semibold py-2.5 rounded-lg transition"
				>
					{loading ? 'Creating account…' : 'Create Account'}
				</button>
			</form>

			<p class="mt-6 text-center text-sm text-gray-500">
				Already have an account? <a href="/login" class="text-indigo-600 hover:underline font-medium">Sign in</a>
			</p>
		</div>
	</div>
</div>
