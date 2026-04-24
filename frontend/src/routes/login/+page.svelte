<script lang="ts">
	import { goto } from '$app/navigation';
	import { auth } from '$lib/stores/auth';
	import { api } from '$lib/api';
	import { toast } from '$lib/stores/toast';

	let email = $state('');
	let password = $state('');
	let totpCode = $state('');
	let loading = $state(false);
	let error = $state('');
	let requires2fa = $state(false);
	let pendingToken = $state('');
	let pendingUser = $state<any>(null);

	function validateEmail(e: string) {
		return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(e);
	}

	async function handleLogin() {
		error = '';
		if (!email || !password) {
			error = 'Email and password are required.';
			return;
		}
		if (!validateEmail(email)) {
			error = 'Please enter a valid email address.';
			return;
		}
		loading = true;
		try {
			const res = await api.auth.login(email, password) as any;
			if (res.requires_2fa) {
				requires2fa = true;
				pendingToken = res.temp_token ?? '';
				pendingUser = res.user ?? null;
			} else {
				auth.login(res.token, res.user);
				toast.success('Welcome back!');
				goto('/dashboard');
			}
		} catch (e: any) {
			error = e.message || 'Login failed.';
			toast.error(error);
		} finally {
			loading = false;
		}
	}

	async function handle2fa() {
		error = '';
		if (totpCode.length !== 6) {
			error = 'Please enter the 6-digit code.';
			return;
		}
		loading = true;
		try {
			const res = await fetch('/api/v1/auth/2fa/verify', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ code: totpCode, temp_token: pendingToken }),
			});
			if (!res.ok) throw new Error(await res.text());
			const data = await res.json();
			auth.login(data.token, data.user);
			toast.success('Welcome back!');
			goto('/dashboard');
		} catch (e: any) {
			error = e.message || '2FA verification failed.';
			toast.error(error);
		} finally {
			loading = false;
		}
	}

	async function handlePasskeyLogin() {
		try {
			// WebAuthn flow – get challenge from server
			const challengeRes = await fetch('/api/v1/auth/passkey/challenge', { method: 'POST' });
			if (!challengeRes.ok) throw new Error('Failed to get passkey challenge');
			const { challenge, rpId } = await challengeRes.json();

			const credential = await navigator.credentials.get({
				publicKey: {
					challenge: Uint8Array.from(atob(challenge), c => c.charCodeAt(0)),
					rpId,
					userVerification: 'preferred',
				},
			}) as PublicKeyCredential;

			const assertionRes = credential.response as AuthenticatorAssertionResponse;
			const verifyRes = await fetch('/api/v1/auth/passkey/verify', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					id: credential.id,
					rawId: btoa(String.fromCharCode(...new Uint8Array(credential.rawId))),
					response: {
						authenticatorData: btoa(String.fromCharCode(...new Uint8Array(assertionRes.authenticatorData))),
						clientDataJSON: btoa(String.fromCharCode(...new Uint8Array(assertionRes.clientDataJSON))),
						signature: btoa(String.fromCharCode(...new Uint8Array(assertionRes.signature))),
					},
					type: credential.type,
				}),
			});
			if (!verifyRes.ok) throw new Error(await verifyRes.text());
			const data = await verifyRes.json();
			auth.login(data.token, data.user);
			toast.success('Signed in with Passkey!');
			goto('/dashboard');
		} catch (e: any) {
			toast.error(e.message || 'Passkey login failed.');
		}
	}
</script>

<div class="min-h-screen bg-gray-50 flex items-center justify-center py-12 px-4">
	<div class="max-w-md w-full">
		<div class="text-center mb-8">
			<h1 class="text-3xl font-bold text-indigo-600">RxForge</h1>
			<p class="mt-2 text-gray-600">Sign in to your account</p>
		</div>

		<div class="bg-white rounded-2xl shadow-sm border border-gray-200 p-8">
			{#if !requires2fa}
				<form onsubmit={(e) => { e.preventDefault(); handleLogin(); }} class="space-y-5">
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
							placeholder="••••••••"
						/>
					</div>

					<button
						type="submit"
						disabled={loading}
						class="w-full bg-indigo-600 hover:bg-indigo-700 disabled:opacity-60 text-white font-semibold py-2.5 rounded-lg transition"
					>
						{loading ? 'Signing in…' : 'Sign in'}
					</button>

					<div class="relative my-4">
						<div class="absolute inset-0 flex items-center"><div class="w-full border-t border-gray-200"></div></div>
						<div class="relative flex justify-center text-sm"><span class="px-3 bg-white text-gray-500">or</span></div>
					</div>

					<button
						type="button"
						onclick={handlePasskeyLogin}
						class="w-full border border-gray-300 hover:bg-gray-50 text-gray-700 font-medium py-2.5 rounded-lg transition flex items-center justify-center gap-2"
					>
						<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 11c0-1.657-1.343-3-3-3S6 9.343 6 11v2a1 1 0 001 1h6a1 1 0 001-1v-2zm0 0V7a5 5 0 0110 0v4M5 21h14a2 2 0 002-2v-5a2 2 0 00-2-2H5a2 2 0 00-2 2v5a2 2 0 002 2z" />
						</svg>
						Sign in with Passkey
					</button>
				</form>

				<p class="mt-6 text-center text-sm text-gray-500">
					Don't have an account? <a href="/register" class="text-indigo-600 hover:underline font-medium">Register</a>
				</p>
			{:else}
				<form onsubmit={(e) => { e.preventDefault(); handle2fa(); }} class="space-y-5">
					<div class="text-center mb-2">
						<h2 class="text-lg font-semibold text-gray-800">Two-Factor Authentication</h2>
						<p class="text-sm text-gray-500 mt-1">Enter the 6-digit code from your authenticator app.</p>
					</div>

					{#if error}
						<div class="bg-red-50 text-red-700 rounded-lg px-4 py-3 text-sm">{error}</div>
					{/if}

					<div>
						<label for="totp" class="block text-sm font-medium text-gray-700 mb-1">TOTP Code</label>
						<input
							id="totp"
							type="text"
							bind:value={totpCode}
							maxlength="6"
							pattern="[0-9]{6}"
							required
							class="w-full px-4 py-2.5 border border-gray-300 rounded-lg focus:ring-2 focus:ring-indigo-500 focus:border-indigo-500 outline-none transition text-center text-2xl tracking-widest"
							placeholder="000000"
						/>
					</div>

					<button
						type="submit"
						disabled={loading}
						class="w-full bg-indigo-600 hover:bg-indigo-700 disabled:opacity-60 text-white font-semibold py-2.5 rounded-lg transition"
					>
						{loading ? 'Verifying…' : 'Verify'}
					</button>

					<button
						type="button"
						onclick={() => { requires2fa = false; totpCode = ''; error = ''; }}
						class="w-full text-gray-500 hover:text-gray-700 text-sm"
					>Back to login</button>
				</form>
			{/if}
		</div>
	</div>
</div>
