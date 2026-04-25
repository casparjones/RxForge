<script lang="ts">
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { auth } from '$lib/stores/auth';
	import { api } from '$lib/api';
	import { toast } from '$lib/stores/toast';
	import RxLogo from '$lib/components/RxLogo.svelte';

	let email = $state('');
	let password = $state('');
	let passwordConfirm = $state('');
	let inviteCode = $state('');
	let loading = $state(false);
	let error = $state('');
	let inviteRequired = $state(false);

	onMount(() => {
		api.auth.info().then(info => { inviteRequired = info.invite_required; }).catch(() => {});
	});

	async function handleRegister() {
		error = '';
		if (!email || !password || !passwordConfirm) { error = 'All fields are required.'; return; }
		if (!email.includes('@')) { error = 'Enter a valid email address.'; return; }
		if (password.length < 8) { error = 'Password must be at least 8 characters.'; return; }
		if (password !== passwordConfirm) { error = 'Passwords do not match.'; return; }
		loading = true;
		try {
			const res = await api.auth.register(email, password, inviteCode || undefined);
			auth.login(res.token, res.user);
			toast.success('Account created! Welcome to RxForge.');
			goto('/dashboard');
		} catch (e: any) {
			error = e.message || 'Registration failed.';
		} finally {
			loading = false;
		}
	}

	function inputStyle(focused = false) {
		return `width:100%; background:#0c0d18; border:1px solid ${focused ? '#7c7cff' : '#2e3247'}; box-shadow:${focused ? '0 0 0 3px #7c7cff22' : 'none'}; border-radius:6px; padding:11px 13px; color:#eef0fa; font-size:14px; font-family:'JetBrains Mono',monospace; outline:none; transition:border-color 120ms, box-shadow 120ms; box-sizing:border-box;`;
	}

	const labelStyle = `font-family:'JetBrains Mono',monospace; font-size:10px; letter-spacing:.08em; text-transform:uppercase; color:#8b8fa8; margin-bottom:6px; display:block;`;
</script>

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

	<!-- LEFT brand panel (lg+) -->
	<div
		class="hidden lg:flex flex-col justify-between flex-[1.1] p-16 xl:p-20 relative z-10"
		style="border-right:1px solid #22253a;"
	>
		<div>
			<RxLogo size={34} color="#eef0fa" accent="#7c7cff" />
		</div>
		<div>
			<div style="font-family:'JetBrains Mono',monospace; font-size:12px; color:#7c7cff; letter-spacing:.18em; margin-bottom:20px;">
				━━ CREATE ACCOUNT
			</div>
			<h1 style="font-size:clamp(40px,4vw,68px); font-weight:500; line-height:.98; letter-spacing:-.035em; margin:0;">
				Forge your<br/><span style="color:#7c7cff;">sync</span> backend.
			</h1>
			<p style="font-size:16px; color:#8b8fa8; line-height:1.55; max-width:400px; margin-top:24px;">
				Self-hosted sync for RxDB apps. Your data stays on your infrastructure.
			</p>
		</div>
		<div style="font-family:'JetBrains Mono',monospace; font-size:11px; color:#8b8fa8;">
			© 2026 RxForge · MIT licensed
		</div>
	</div>

	<!-- RIGHT register panel -->
	<div
		class="flex-1 flex items-center justify-center p-6 sm:p-12 relative z-10"
		style="background:#100d17;"
	>
		<div class="w-full max-w-[420px]">
			<div class="lg:hidden flex justify-center mb-8">
				<RxLogo size={30} color="#eef0fa" accent="#7c7cff" />
			</div>

			<div
				class="rounded-xl p-8 sm:p-10"
				style="background:#161829; border:1px solid #2e3247;"
			>
				<div class="mb-7">
					<div style="font-family:'JetBrains Mono',monospace; font-size:10px; color:#7c7cff; letter-spacing:.15em; margin-bottom:10px;">
						─ REGISTER
					</div>
					<div style="font-size:24px; font-weight:600; letter-spacing:-.02em;">Create account</div>
					<div style="color:#8b8fa8; font-size:13px; margin-top:5px;">
						Your CouchDB instance is provisioned on first sync.
					</div>
				</div>

				<form onsubmit={(e) => { e.preventDefault(); handleRegister(); }} class="flex flex-col gap-4">
					{#if error}
						<div style="font-family:'JetBrains Mono',monospace; font-size:12px; color:#ff9ab0; background:#2a1422; border:1px solid #4a2034; border-radius:4px; padding:8px 12px;">
							! {error}
						</div>
					{/if}

					<div>
						<label for="email" style={labelStyle}>Email</label>
						<input
							id="email" type="email" bind:value={email} required
							placeholder="dev@example.com"
							style={inputStyle()}
							onfocus={(e) => { (e.target as HTMLInputElement).style.borderColor='#7c7cff'; (e.target as HTMLInputElement).style.boxShadow='0 0 0 3px #7c7cff22'; }}
							onblur={(e) => { (e.target as HTMLInputElement).style.borderColor='#2e3247'; (e.target as HTMLInputElement).style.boxShadow='none'; }}
						/>
					</div>

					<div>
						<label for="password" style={labelStyle}>Password</label>
						<input
							id="password" type="password" bind:value={password} required
							placeholder="Min. 8 characters"
							style={inputStyle()}
							onfocus={(e) => { (e.target as HTMLInputElement).style.borderColor='#7c7cff'; (e.target as HTMLInputElement).style.boxShadow='0 0 0 3px #7c7cff22'; }}
							onblur={(e) => { (e.target as HTMLInputElement).style.borderColor='#2e3247'; (e.target as HTMLInputElement).style.boxShadow='none'; }}
						/>
					</div>

					<div>
						<label for="confirm" style={labelStyle}>Confirm Password</label>
						<input
							id="confirm" type="password" bind:value={passwordConfirm} required
							placeholder="••••••••"
							style={inputStyle()}
							onfocus={(e) => { (e.target as HTMLInputElement).style.borderColor='#7c7cff'; (e.target as HTMLInputElement).style.boxShadow='0 0 0 3px #7c7cff22'; }}
							onblur={(e) => { (e.target as HTMLInputElement).style.borderColor='#2e3247'; (e.target as HTMLInputElement).style.boxShadow='none'; }}
						/>
						{#if passwordConfirm && password !== passwordConfirm}
							<p style="font-family:'JetBrains Mono',monospace; font-size:11px; color:#ff9ab0; margin-top:5px;">passwords do not match</p>
						{/if}
					</div>

					{#if inviteRequired}
					<div>
						<label for="invite" style={labelStyle}>Invite Code</label>
						<input
							id="invite" type="text" bind:value={inviteCode} required
							placeholder="Enter invite code"
							style={inputStyle()}
							onfocus={(e) => { (e.target as HTMLInputElement).style.borderColor='#7c7cff'; (e.target as HTMLInputElement).style.boxShadow='0 0 0 3px #7c7cff22'; }}
							onblur={(e) => { (e.target as HTMLInputElement).style.borderColor='#2e3247'; (e.target as HTMLInputElement).style.boxShadow='none'; }}
						/>
					</div>
					{/if}

					<button
						type="submit"
						disabled={loading}
						style="background:{loading ? '#5a5aee' : '#7c7cff'}; color:#05050f; border:none; border-radius:6px; padding:13px; font-family:'Space Grotesk',sans-serif; font-weight:600; font-size:14px; cursor:{loading ? 'wait' : 'pointer'}; transition:background 120ms; opacity:{loading ? .8 : 1}; margin-top:2px;"
					>
						{loading ? 'Creating account…' : 'Create Account'}
					</button>
				</form>

				<div class="mt-5 text-center" style="font-family:'JetBrains Mono',monospace; font-size:11px; color:#8b8fa8;">
					Already have an account? <a href="/login" style="color:#eef0fa; text-decoration:none; border-bottom:1px dotted #8b8fa8;">Sign in</a>
				</div>
			</div>
		</div>
	</div>
</div>
