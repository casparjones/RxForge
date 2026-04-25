<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';
	import { auth } from '$lib/stores/auth';
	import { api } from '$lib/api';
	import { toast } from '$lib/stores/toast';
	import RxLogo from '$lib/components/RxLogo.svelte';
	import { PUBLIC_APP_DOMAIN } from '$env/static/public';

	const domain = PUBLIC_APP_DOMAIN || 'rxforge.de';

	let email = $state('');
	let password = $state('');
	let totpCode = $state('');
	let loading = $state(false);
	let error = $state('');
	let requires2fa = $state(false);
	let pendingToken = $state('');

	function getReturnTo(): string {
		const rt = $page.url.searchParams.get('return_to');
		if (rt) return decodeURIComponent(rt);
		return '/dashboard';
	}

	async function handleLogin() {
		error = '';
		if (!email || !password) { error = 'Email and password are required.'; return; }
		if (!email.includes('@')) { error = 'Enter a valid email address.'; return; }
		loading = true;
		try {
			const res = await api.auth.login(email, password) as any;
			if (res.requires_2fa) {
				requires2fa = true;
				pendingToken = res.temp_token ?? '';
			} else {
				auth.login(res.token, res.user);
				toast.success('Welcome back!');
				window.location.href = getReturnTo();
			}
		} catch (e: any) {
			error = e.message || 'Authentication failed.';
		} finally {
			loading = false;
		}
	}

	async function handle2fa() {
		error = '';
		if (totpCode.length !== 6) { error = 'Enter the 6-digit code.'; return; }
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
			window.location.href = getReturnTo();
		} catch (e: any) {
			error = e.message || '2FA verification failed.';
		} finally {
			loading = false;
		}
	}

	const features = [
		{ n: '01', k: 'OAUTH 2.0',        v: 'Standard authorization code flow with consent screen.' },
		{ n: '02', k: 'PER-USER COUCHDB', v: 'Provisioned on first login. Fully isolated.' },
		{ n: '03', k: 'RXDB PLUGIN',      v: 'Drop-in replication for your client app.' },
		{ n: '04', k: 'SELF-HOSTED',      v: 'Your servers, your keys, your data.' },
	];
</script>

<!-- Always dark – auth pages use the brand palette -->
<div
	class="min-h-screen flex"
	style="background:#0e0f1a; color:#eef0fa; font-family:'Space Grotesk',system-ui,sans-serif;"
>
	<!-- Corner ticks -->
	<div class="pointer-events-none fixed inset-0 z-0" aria-hidden="true">
		<span class="absolute top-8 left-8 w-7 h-7 border-t border-l" style="border-color:#2e3247"></span>
		<span class="absolute top-8 right-8 w-7 h-7 border-t border-r" style="border-color:#2e3247"></span>
		<span class="absolute bottom-8 left-8 w-7 h-7 border-b border-l" style="border-color:#2e3247"></span>
		<span class="absolute bottom-8 right-8 w-7 h-7 border-b border-r" style="border-color:#2e3247"></span>
	</div>

	<!-- LEFT — brand panel (hidden on mobile) -->
	<div
		class="hidden lg:flex flex-col justify-between flex-[1.3] p-16 xl:p-24 relative z-10"
		style="border-right:1px solid #22253a;"
	>
		<!-- Top: lockup + status -->
		<div class="flex items-center justify-between">
			<RxLogo size={36} color="#eef0fa" accent="#7c7cff" />
			<div class="flex items-center gap-2" style="font-family:'JetBrains Mono',monospace; font-size:11px; color:#8b8fa8;">
				<span class="w-1.5 h-1.5 rounded-full bg-green-400" style="box-shadow:0 0 6px #4ade80;"></span>
				<span>{domain}</span>
				<span style="opacity:.4">·</span>
				<span>v1.4.2</span>
				<span style="opacity:.4">·</span>
				<span>eu-fra</span>
			</div>
		</div>

		<!-- Middle: hero -->
		<div>
			<div style="font-family:'JetBrains Mono',monospace; font-size:12px; color:#7c7cff; letter-spacing:.18em; margin-bottom:24px;">
				━━ AUTH · v1.4.2
			</div>
			<h1 style="font-size:clamp(48px,5vw,80px); font-weight:500; line-height:.98; letter-spacing:-.035em; margin:0; max-width:640px;">
				Self-hosted sync<br/>
				<span style="color:#8b8fa8;">for </span><span style="color:#7c7cff;">RxDB</span><span style="color:#8b8fa8;"> apps.</span>
			</h1>
			<p style="font-size:17px; color:#8b8fa8; line-height:1.55; max-width:480px; margin-top:28px; margin-bottom:52px;">
				OAuth 2.0, per-user CouchDB provisioning, and a TypeScript plugin for seamless replication. Forge your own backend.
			</p>

			<!-- Feature list -->
			<div class="flex flex-col gap-5">
				{#each features as f}
					<div class="flex gap-5">
						<div style="font-family:'JetBrains Mono',monospace; font-size:10px; color:#7c7cff; letter-spacing:.1em; min-width:120px; padding-top:3px;">
							{f.n} · {f.k}
						</div>
						<div style="font-size:15px; color:#eef0fa; line-height:1.45; max-width:320px;">{f.v}</div>
					</div>
				{/each}
			</div>
		</div>

		<!-- Bottom footer -->
		<div class="flex justify-between" style="font-family:'JetBrains Mono',monospace; font-size:11px; color:#8b8fa8;">
			<span>© 2026 RxForge · MIT licensed</span>
			<span>{domain}</span>
		</div>
	</div>

	<!-- RIGHT — login card panel -->
	<div
		class="flex-1 flex items-center justify-center p-6 sm:p-12 relative z-10"
		style="background:#100d17;"
	>
		<div class="w-full max-w-[420px]">

			<!-- Mobile-only logo -->
			<div class="lg:hidden flex justify-center mb-8">
				<RxLogo size={32} color="#eef0fa" accent="#7c7cff" />
			</div>

			{#if !requires2fa}
				<!-- Login card -->
				<div
					class="rounded-xl p-8 sm:p-10"
					style="background:#161829; border:1px solid #2e3247;"
				>
					<div class="mb-7">
						<div style="font-family:'JetBrains Mono',monospace; font-size:10px; color:#7c7cff; letter-spacing:.15em; margin-bottom:10px;">
							─ SIGN IN
						</div>
						<div style="font-size:24px; font-weight:600; letter-spacing:-.02em;">Welcome back</div>
						<div style="color:#8b8fa8; font-size:13px; margin-top:5px;">
							Authenticate to sync your local-first data.
						</div>
					</div>

					<form onsubmit={(e) => { e.preventDefault(); handleLogin(); }} class="flex flex-col gap-4">
						{#if error}
							<div style="font-family:'JetBrains Mono',monospace; font-size:12px; color:#ff9ab0; background:#2a1422; border:1px solid #4a2034; border-radius:4px; padding:8px 12px;">
								! {error}
							</div>
						{/if}

						<div>
							<label
								for="email"
								style="font-family:'JetBrains Mono',monospace; font-size:10px; letter-spacing:.08em; text-transform:uppercase; color:#8b8fa8; margin-bottom:6px; display:block;"
							>Email</label>
							<input
								id="email"
								type="email"
								bind:value={email}
								required
								placeholder="dev@example.com"
								style="width:100%; background:#0c0d18; border:1px solid #2e3247; border-radius:6px; padding:11px 13px; color:#eef0fa; font-size:14px; font-family:'JetBrains Mono',monospace; outline:none; transition:border-color 120ms, box-shadow 120ms; box-sizing:border-box;"
								onfocus={(e) => { (e.target as HTMLInputElement).style.borderColor='#7c7cff'; (e.target as HTMLInputElement).style.boxShadow='0 0 0 3px #7c7cff22'; }}
								onblur={(e) => { (e.target as HTMLInputElement).style.borderColor='#2e3247'; (e.target as HTMLInputElement).style.boxShadow='none'; }}
							/>
						</div>

						<div>
							<div class="flex justify-between items-baseline" style="margin-bottom:6px;">
								<label
									for="password"
									style="font-family:'JetBrains Mono',monospace; font-size:10px; letter-spacing:.08em; text-transform:uppercase; color:#8b8fa8;"
								>Password</label>
							</div>
							<input
								id="password"
								type="password"
								bind:value={password}
								required
								placeholder="••••••••••"
								style="width:100%; background:#0c0d18; border:1px solid #2e3247; border-radius:6px; padding:11px 13px; color:#eef0fa; font-size:14px; font-family:'JetBrains Mono',monospace; outline:none; transition:border-color 120ms, box-shadow 120ms; box-sizing:border-box;"
								onfocus={(e) => { (e.target as HTMLInputElement).style.borderColor='#7c7cff'; (e.target as HTMLInputElement).style.boxShadow='0 0 0 3px #7c7cff22'; }}
								onblur={(e) => { (e.target as HTMLInputElement).style.borderColor='#2e3247'; (e.target as HTMLInputElement).style.boxShadow='none'; }}
							/>
						</div>

						<button
							type="submit"
							disabled={loading}
							style="background:{loading ? '#5a5aee' : '#7c7cff'}; color:#05050f; border:none; border-radius:6px; padding:13px; font-family:'Space Grotesk',sans-serif; font-weight:600; font-size:14px; letter-spacing:.01em; cursor:{loading ? 'wait' : 'pointer'}; transition:background 120ms, transform 80ms; opacity:{loading ? .8 : 1}; margin-top:4px;"
							onmousedown={(e) => { if (!loading) (e.currentTarget as HTMLButtonElement).style.transform='scale(0.98)'; }}
							onmouseup={(e) => { (e.currentTarget as HTMLButtonElement).style.transform='scale(1)'; }}
						>
							{loading ? 'Authenticating…' : 'Sign in to RxForge'}
						</button>
					</form>

					<div
						class="mt-5 text-center"
						style="font-family:'JetBrains Mono',monospace; font-size:11px; color:#8b8fa8;"
					>
						New here? <a href="/register" style="color:#eef0fa; text-decoration:none; border-bottom:1px dotted #8b8fa8;">Create an account</a>
					</div>
				</div>

			{:else}
				<!-- 2FA card -->
				<div
					class="rounded-xl p-8 sm:p-10"
					style="background:#161829; border:1px solid #2e3247;"
				>
					<div class="mb-7">
						<div style="font-family:'JetBrains Mono',monospace; font-size:10px; color:#7c7cff; letter-spacing:.15em; margin-bottom:10px;">
							─ TWO-FACTOR AUTH
						</div>
						<div style="font-size:22px; font-weight:600; letter-spacing:-.02em;">Enter your code</div>
						<div style="color:#8b8fa8; font-size:13px; margin-top:5px;">
							Check your authenticator app for a 6-digit code.
						</div>
					</div>

					<form onsubmit={(e) => { e.preventDefault(); handle2fa(); }} class="flex flex-col gap-4">
						{#if error}
							<div style="font-family:'JetBrains Mono',monospace; font-size:12px; color:#ff9ab0; background:#2a1422; border:1px solid #4a2034; border-radius:4px; padding:8px 12px;">
								! {error}
							</div>
						{/if}

						<div>
							<label
								for="totp"
								style="font-family:'JetBrains Mono',monospace; font-size:10px; letter-spacing:.08em; text-transform:uppercase; color:#8b8fa8; margin-bottom:6px; display:block;"
							>TOTP Code</label>
							<input
								id="totp"
								type="text"
								bind:value={totpCode}
								maxlength="6"
								pattern="[0-9]{6}"
								required
								placeholder="000000"
								style="width:100%; background:#0c0d18; border:1px solid #2e3247; border-radius:6px; padding:11px 13px; color:#eef0fa; font-size:22px; font-family:'JetBrains Mono',monospace; outline:none; text-align:center; letter-spacing:.25em; box-sizing:border-box;"
								onfocus={(e) => { (e.target as HTMLInputElement).style.borderColor='#7c7cff'; }}
								onblur={(e) => { (e.target as HTMLInputElement).style.borderColor='#2e3247'; }}
							/>
						</div>

						<button
							type="submit"
							disabled={loading}
							style="background:#7c7cff; color:#05050f; border:none; border-radius:6px; padding:13px; font-family:'Space Grotesk',sans-serif; font-weight:600; font-size:14px; cursor:{loading ? 'wait' : 'pointer'}; opacity:{loading ? .8 : 1};"
						>
							{loading ? 'Verifying…' : 'Verify Code'}
						</button>

						<button
							type="button"
							onclick={() => { requires2fa = false; totpCode = ''; error = ''; }}
							style="background:transparent; border:none; color:#8b8fa8; font-family:'JetBrains Mono',monospace; font-size:12px; cursor:pointer; padding:8px;"
						>← Back to login</button>
					</form>
				</div>
			{/if}
		</div>
	</div>
</div>
