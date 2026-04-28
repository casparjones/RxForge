<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	import { get } from 'svelte/store';
	import { api } from '$lib/api';
	import { parseOrigins } from '$lib/origins';
	import { toast } from '$lib/stores/toast';
	import { t } from '$lib/i18n';
	import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';

	const appId = $derived($page.params.id!);

	let app = $state<any | null>(null);
	let loading = $state(true);
	let loadError = $state('');

	let editName = $state('');
	let editAuthType = $state('oauth');
	let editDbScope = $state('isolated');
	let editRedirects = $state('');
	let saving = $state(false);

	let confirmOpen = $state(false);

	// Token management
	let tokens = $state<any[]>([]);
	let loadingTokens = $state(false);
	let showAddToken = $state(false);
	let newTokenName = $state('');
	let newTokenOrigins = $state('');
	let creatingToken = $state(false);
	let revealedToken = $state<{ token: string; name: string } | null>(null);
	let confirmToken = $state<any | null>(null);

	onMount(async () => {
		try {
			app = await api.apps.get(appId);
			editName = app.name;
			editAuthType = app.auth_type ?? 'oauth';
			editDbScope = app.db_scope ?? 'isolated';
			editRedirects = (app.redirect_uris ?? []).join('\n');
			if (app.auth_type === 'token') await loadTokens();
		} catch (e: any) {
			loadError = e.message;
		} finally {
			loading = false;
		}
	});

	async function loadTokens() {
		loadingTokens = true;
		try { tokens = await api.apps.tokens.list(appId); }
		catch { tokens = []; }
		finally { loadingTokens = false; }
	}

	async function createToken() {
		creatingToken = true;
		try {
			const origins = parseOrigins(newTokenOrigins);
			const res = await api.apps.tokens.create(appId, {
				name: newTokenName.trim() || undefined,
				allowed_origins: origins,
			});
			tokens = [res, ...tokens];
			showAddToken = false;
			revealedToken = { token: res.token, name: res.name };
		} catch (e: any) {
			toast.error('Error: ' + e.message);
		} finally {
			creatingToken = false;
		}
	}

	async function revokeToken(tokenId: string) {
		try {
			await api.apps.tokens.revoke(appId, tokenId);
			tokens = tokens.map(tk => tk.id === tokenId ? { ...tk, revoked: true } : tk);
			toast.success(get(t)('tokens.revokedMsg'));
		} catch (e: any) {
			toast.error('Error: ' + e.message);
		}
	}

	let editToken = $state<any | null>(null);
	let editTokenName = $state('');
	let editTokenOrigins = $state('');
	let savingToken = $state(false);
	let confirmPurgeToken = $state<any | null>(null);

	async function openEditToken(tok: any) {
		editToken = tok;
		editTokenName = tok.name;
		editTokenOrigins = (tok.allowed_origins ?? []).join('\n');
	}

	async function saveEditToken() {
		if (!editToken) return;
		savingToken = true;
		try {
			const origins = parseOrigins(editTokenOrigins);
			await api.apps.tokens.update(appId, editToken.id, {
				name: editTokenName.trim() || undefined,
				allowed_origins: origins,
			});
			tokens = tokens.map(tk => tk.id === editToken!.id ? { ...tk, name: editTokenName, allowed_origins: origins } : tk);
			editToken = null;
			toast.success(get(t)('tokens.updated'));
		} catch (e: any) {
			toast.error('Error: ' + e.message);
		} finally {
			savingToken = false;
		}
	}

	async function purgeToken(tokenId: string) {
		try {
			await api.apps.tokens.purge(appId, tokenId);
			tokens = tokens.filter(tk => tk.id !== tokenId);
			toast.success(get(t)('tokens.deleted'));
		} catch (e: any) {
			toast.error('Error: ' + e.message);
		}
	}

	async function save() {
		saving = true;
		try {
			const redirectUris = editRedirects.split('\n').map((s: string) => s.trim()).filter(Boolean);
			const updated = await api.apps.update(appId, {
				name: editName,
				auth_type: editAuthType,
				db_scope: editDbScope,
				redirect_uris: redirectUris,
			});
			app = { ...app, ...updated };
			toast.success(get(t)('apps.appSaved'));
		} catch (e: any) {
			toast.error('Error: ' + e.message);
		} finally {
			saving = false;
		}
	}

	async function doDelete() {
		try {
			await api.apps.delete(appId);
			toast.success(get(t)('apps.appDeleted'));
			goto('/dashboard/apps');
		} catch (e: any) {
			toast.error('Error: ' + e.message);
		}
	}
</script>

<svelte:head><title>{app?.name ?? 'Edit App'} – RxForge</title></svelte:head>

<div class="min-h-screen flex flex-col" style="background:var(--c-bg,#05050f);">

	<!-- Top bar -->
	<div class="flex items-center gap-3 px-6 py-4 shrink-0" style="border-bottom:1px solid var(--c-border); background:var(--c-surface);">
		<button
			onclick={() => goto('/dashboard/apps')}
			class="text-sm flex items-center gap-1.5 transition"
			style="color:var(--c-muted);"
			onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.color='var(--c-text)'; }}
			onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.color='var(--c-muted)'; }}
		>
			<svg class="w-4 h-4" viewBox="0 0 20 20" fill="currentColor"><path fill-rule="evenodd" d="M9.707 16.707a1 1 0 01-1.414 0l-6-6a1 1 0 010-1.414l6-6a1 1 0 011.414 1.414L5.414 9H17a1 1 0 110 2H5.414l4.293 4.293a1 1 0 010 1.414z" clip-rule="evenodd"/></svg>
			{$t('common.back')}
		</button>
		{#if app}
			<span style="color:var(--c-border);">/</span>
			<span class="text-sm font-semibold" style="color:var(--c-text);">{app.name}</span>
			<span style="color:var(--c-border);">/</span>
		{/if}
		<span class="text-sm" style="color:var(--c-muted);">Edit</span>
	</div>

	<!-- Content -->
	<div class="flex-1 flex items-start justify-center px-4 py-10">

		{#if loading}
			<div class="flex justify-center py-20">
				<div class="w-6 h-6 rounded-full border-2 animate-spin" style="border-color:#7c7cff; border-top-color:transparent;"></div>
			</div>

		{:else if loadError}
			<div class="text-center py-20">
				<p class="text-sm mb-3" style="color:#f87171;">{loadError}</p>
				<button onclick={() => goto('/dashboard/apps')} class="text-sm" style="color:#7c7cff;">← {$t('common.back')}</button>
			</div>

		{:else}
			<div style="width:100%; max-width:560px;">

				<!-- Card -->
				<div style="background:var(--c-surface); border:1px solid var(--c-border); border-radius:16px; overflow:hidden; box-shadow:0 8px 32px rgba(0,0,0,.3);">

					<!-- Header -->
					<div class="px-6 py-5" style="border-bottom:1px solid var(--c-border);">
						<h1 style="font-size:18px; font-weight:600; color:var(--c-text);">{$t('apps.editTitle', { name: app.name })}</h1>
						<p class="text-xs mt-1 font-mono" style="color:var(--c-muted);">{app.client_id}</p>
					</div>

					<!-- Fields -->
					<div class="px-6 py-6 space-y-6">

						<!-- Name -->
						<div>
							<label class="block text-xs font-semibold uppercase tracking-wide mb-1.5" style="color:var(--c-muted);">{$t('apps.appName')}</label>
							<input
								type="text"
								bind:value={editName}
								class="w-full px-3 py-2 text-sm rounded-lg outline-none"
								style="background:var(--c-bg); border:1px solid var(--c-border); color:var(--c-text);"
								onfocus={(e) => { (e.currentTarget as HTMLElement).style.borderColor='#7c7cff'; }}
								onblur={(e) => { (e.currentTarget as HTMLElement).style.borderColor='var(--c-border)'; }}
							/>
						</div>

						<!-- Auth Type -->
						<div>
							<p class="text-xs font-semibold uppercase tracking-wide mb-2" style="color:var(--c-muted);">{$t('apps.authType')}</p>
							<div class="grid grid-cols-2 gap-3">
								<label class="flex flex-col gap-1 rounded-lg p-3 cursor-pointer" style="{editAuthType === 'oauth' ? 'border:1px solid #7c7cff; background:rgba(124,124,255,.06);' : 'border:1px solid var(--c-border);'}">
									<input type="radio" bind:group={editAuthType} value="oauth" class="sr-only" />
									<span class="font-medium text-sm" style="color:var(--c-text);">{$t('apps.authOAuth')}</span>
									<span class="text-xs" style="color:var(--c-muted);">{$t('apps.authOAuthDesc')}</span>
								</label>
								<label class="flex flex-col gap-1 rounded-lg p-3 cursor-pointer" style="{editAuthType === 'token' ? 'border:1px solid #fbbf24; background:rgba(251,191,36,.06);' : 'border:1px solid var(--c-border);'}">
									<input type="radio" bind:group={editAuthType} value="token" class="sr-only" />
									<span class="font-medium text-sm" style="color:var(--c-text);">{$t('apps.authToken')}</span>
									<span class="text-xs" style="color:var(--c-muted);">{$t('apps.authTokenDesc')}</span>
								</label>
							</div>
							{#if editAuthType === 'token'}
								<p class="mt-2 text-xs rounded-lg px-3 py-2" style="color:#fbbf24; background:rgba(251,191,36,.06); border:1px solid rgba(251,191,36,.2);">
									{$t('apps.authTokenWarning')}
								</p>
							{/if}
						</div>

						<!-- DB Scope -->
						<div>
							<p class="text-xs font-semibold uppercase tracking-wide mb-2" style="color:var(--c-muted);">{$t('apps.dbScope')}</p>
							<div class="grid grid-cols-2 gap-3">
								<label class="flex flex-col gap-1 rounded-lg p-3 cursor-pointer" style="{editDbScope === 'isolated' ? 'border:1px solid #7c7cff; background:rgba(124,124,255,.06);' : 'border:1px solid var(--c-border);'}">
									<input type="radio" bind:group={editDbScope} value="isolated" class="sr-only" />
									<span class="font-medium text-sm" style="color:var(--c-text);">{$t('apps.dbIsolated')}</span>
									<span class="text-xs" style="color:var(--c-muted);">{$t('apps.dbIsolatedDesc')}</span>
								</label>
								<label class="flex flex-col gap-1 rounded-lg p-3 cursor-pointer" style="{editDbScope === 'shared' ? 'border:1px solid #f87171; background:rgba(248,113,113,.06);' : 'border:1px solid var(--c-border);'}">
									<input type="radio" bind:group={editDbScope} value="shared" class="sr-only" />
									<span class="font-medium text-sm" style="color:var(--c-text);">{$t('apps.dbShared')}</span>
									<span class="text-xs" style="color:var(--c-muted);">{$t('apps.dbSharedDesc')}</span>
								</label>
							</div>
							{#if editDbScope === 'shared'}
								<p class="mt-2 text-xs rounded-lg px-3 py-2" style="color:#f87171; background:rgba(248,113,113,.06); border:1px solid rgba(248,113,113,.2);">
									{$t('apps.dbSharedWarning')}
								</p>
							{/if}
						</div>

						<!-- Redirect URIs -->
						{#if editAuthType === 'oauth'}
							<div>
								<label class="block text-xs font-semibold uppercase tracking-wide mb-1.5" style="color:var(--c-muted);">
									{$t('apps.redirectUris')} <span class="normal-case font-normal">({$t('apps.redirectUrisHint')})</span>
								</label>
								<textarea
									bind:value={editRedirects}
									rows="3"
									class="w-full px-3 py-2 text-sm rounded-lg outline-none resize-none"
									style="background:var(--c-bg); border:1px solid var(--c-border); color:var(--c-text); font-family:'JetBrains Mono',monospace; font-size:12px;"
									placeholder="https://myapp.com/callback"
									onfocus={(e) => { (e.currentTarget as HTMLElement).style.borderColor='#7c7cff'; }}
									onblur={(e) => { (e.currentTarget as HTMLElement).style.borderColor='var(--c-border)'; }}
								></textarea>
							</div>
						{/if}

					</div>

					<!-- ── Token Management (nur für token-Apps) ── -->
					{#if editAuthType === 'token'}
					<div class="px-6 py-5 space-y-3" style="border-top:1px solid var(--c-border);">
						<div class="flex items-center justify-between">
							<p class="text-xs font-semibold uppercase tracking-wide" style="color:var(--c-muted);">{$t('tokens.publicTokens')}</p>
							<button
								onclick={() => { newTokenName = ''; newTokenOrigins = ''; showAddToken = true; }}
								class="text-xs font-semibold px-3 py-1.5 rounded-lg transition"
								style="background:#fbbf24; color:#1a1200;"
								onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background='#fcd34d'; }}
								onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background='#fbbf24'; }}
							>{$t('tokens.addToken')}</button>
						</div>

						{#if loadingTokens}
							<p class="text-sm" style="color:var(--c-muted);">{$t('tokens.loading')}</p>
						{:else if !tokens.length}
							<p class="text-sm py-2" style="color:var(--c-muted);">{$t('tokens.noTokens')}</p>
						{:else}
							<div class="space-y-2">
								{#each tokens as tok (tok.id)}
									<div class="rounded-lg px-4 py-3 flex items-start justify-between gap-3" style="background:var(--c-bg); border:1px solid {tok.revoked ? 'var(--c-border)' : 'rgba(251,191,36,.2)'}; opacity:{tok.revoked ? .5 : 1};">
										<div class="min-w-0">
											<div class="flex items-center gap-2 flex-wrap">
												<code class="text-xs font-mono" style="color:#fbbf24;">{tok.token_prefix}…</code>
												<span class="text-xs font-medium" style="color:var(--c-text);">{tok.name}</span>
												{#if tok.revoked}
													<span class="text-xs px-1.5 py-0.5 rounded" style="background:rgba(248,113,113,.12); color:#f87171;">{$t('tokens.revoked')}</span>
												{:else}
													<span class="text-xs px-1.5 py-0.5 rounded" style="background:rgba(74,222,128,.1); color:#4ade80;">{$t('tokens.active')}</span>
												{/if}
											</div>
											{#if tok.allowed_origins?.length}
												<p class="text-xs mt-1 font-mono" style="color:var(--c-muted);">{$t('tokens.origins')} {tok.allowed_origins.join(', ')}</p>
											{:else}
												<p class="text-xs mt-1" style="color:var(--c-muted);">{$t('tokens.allOriginsAllowed')}</p>
											{/if}
											{#if tok.last_used_at}
												<p class="text-xs mt-0.5" style="color:var(--c-muted);">{$t('tokens.lastUsed')} {new Date(tok.last_used_at).toLocaleDateString('en')}</p>
											{/if}
										</div>
										<div class="flex gap-1.5 shrink-0">
											{#if !tok.revoked}
												<button
													onclick={() => openEditToken(tok)}
													class="text-xs px-2 py-1 rounded transition"
													style="color:#7c7cff; border:1px solid rgba(124,124,255,.25); background:transparent;"
													onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background='rgba(124,124,255,.08)'; }}
													onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background='transparent'; }}
												>{$t('tokens.edit')}</button>
												<button
													onclick={() => { confirmToken = tok; }}
													class="text-xs px-2 py-1 rounded transition"
													style="color:#f87171; border:1px solid rgba(248,113,113,.25); background:transparent;"
													onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background='rgba(248,113,113,.08)'; }}
													onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background='transparent'; }}
												>{$t('tokens.revoke')}</button>
											{:else}
												<button
													onclick={() => { confirmPurgeToken = tok; }}
													class="text-xs px-2 py-1 rounded transition"
													style="color:#f87171; border:1px solid rgba(248,113,113,.25); background:transparent;"
													onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background='rgba(248,113,113,.08)'; }}
													onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background='transparent'; }}
												>{$t('tokens.delete')}</button>
											{/if}
										</div>
									</div>
								{/each}
							</div>
						{/if}
					</div>
					{/if}

					<!-- Footer -->
					<div class="flex items-center justify-between px-6 py-4" style="border-top:1px solid var(--c-border);">
						<button
							onclick={() => { confirmOpen = true; }}
							class="text-sm font-medium px-3 py-1.5 rounded-lg transition"
							style="color:#f87171; border:1px solid rgba(248,113,113,.25); background:transparent;"
							onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background='rgba(248,113,113,.08)'; }}
							onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background='transparent'; }}
						>{$t('apps.deleteApp')}</button>
						<div class="flex gap-2">
							<button
								onclick={() => goto('/dashboard/apps')}
								class="text-sm font-medium px-4 py-1.5 rounded-lg transition"
								style="border:1px solid var(--c-border); color:var(--c-muted); background:transparent;"
								onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.color='var(--c-text)'; }}
								onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.color='var(--c-muted)'; }}
							>{$t('apps.cancel')}</button>
							<button
								onclick={save}
								disabled={saving}
								class="text-sm font-semibold px-4 py-1.5 rounded-lg disabled:opacity-60 transition"
								style="background:#7c7cff; color:#05050f;"
								onmouseenter={(e) => { if (!saving) (e.currentTarget as HTMLElement).style.background='#9090ff'; }}
								onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background='#7c7cff'; }}
							>{saving ? $t('apps.saving') : $t('common.save')}</button>
						</div>
					</div>

				</div>
			</div>
		{/if}

	</div>
</div>

<ConfirmDialog
	open={confirmOpen}
	title={$t('apps.deleteApp')}
	message={$t('apps.deleteAppConfirm', { name: app?.name ?? '' })}
	confirmLabel={$t('apps.deleteApp')}
	destructive={true}
	onConfirm={() => { confirmOpen = false; doDelete(); }}
	onCancel={() => { confirmOpen = false; }}
/>

<ConfirmDialog
	open={!!confirmToken}
	title={$t('tokens.revokeTitle')}
	message={$t('tokens.revokeConfirm', { name: confirmToken?.name ?? '' })}
	confirmLabel={$t('tokens.revoke')}
	destructive={true}
	onConfirm={() => { const id = confirmToken!.id; confirmToken = null; revokeToken(id); }}
	onCancel={() => { confirmToken = null; }}
/>

<ConfirmDialog
	open={!!confirmPurgeToken}
	title={$t('tokens.deleteTitle')}
	message={$t('tokens.deleteConfirm', { name: confirmPurgeToken?.name ?? '' })}
	confirmLabel={$t('tokens.delete')}
	destructive={true}
	onConfirm={() => { const id = confirmPurgeToken!.id; confirmPurgeToken = null; purgeToken(id); }}
	onCancel={() => { confirmPurgeToken = null; }}
/>

<!-- Edit Token Modal -->
{#if editToken}
<div class="fixed inset-0 z-50 flex items-center justify-center p-4" style="background:rgba(0,0,0,.6);">
	<div style="background:var(--c-surface); border:1px solid var(--c-border); border-radius:16px; width:100%; max-width:420px; box-shadow:0 16px 48px rgba(0,0,0,.5);">
		<div class="px-6 py-5" style="border-bottom:1px solid var(--c-border);">
			<h2 class="text-base font-semibold" style="color:var(--c-text);">{$t('tokens.editTitle')}</h2>
		</div>
		<div class="px-6 py-5 space-y-4">
			<div>
				<label class="block text-xs font-semibold uppercase tracking-wide mb-1.5" style="color:var(--c-muted);">{$t('tokens.name')}</label>
				<input
					type="text"
					bind:value={editTokenName}
					placeholder={$t('tokens.namePlaceholder')}
					class="w-full px-3 py-2 text-sm rounded-lg outline-none"
					style="background:var(--c-bg); border:1px solid var(--c-border); color:var(--c-text);"
					onfocus={(e) => { (e.currentTarget as HTMLElement).style.borderColor='#fbbf24'; }}
					onblur={(e) => { (e.currentTarget as HTMLElement).style.borderColor='var(--c-border)'; }}
				/>
			</div>
			<div>
				<label class="block text-xs font-semibold uppercase tracking-wide mb-1.5" style="color:var(--c-muted);">
					{$t('tokens.allowedOrigins')} <span class="normal-case font-normal">{$t('tokens.originsHint')}</span>
				</label>
				<textarea
					bind:value={editTokenOrigins}
					rows="3"
					placeholder="https://myapp.com"
					class="w-full px-3 py-2 text-sm rounded-lg outline-none resize-none"
					style="background:var(--c-bg); border:1px solid var(--c-border); color:var(--c-text); font-family:'JetBrains Mono',monospace; font-size:12px;"
					onfocus={(e) => { (e.currentTarget as HTMLElement).style.borderColor='#fbbf24'; }}
					onblur={(e) => { (e.currentTarget as HTMLElement).style.borderColor='var(--c-border)'; }}
				></textarea>
			</div>
		</div>
		<div class="flex justify-end gap-2 px-6 py-4" style="border-top:1px solid var(--c-border);">
			<button
				onclick={() => { editToken = null; }}
				class="text-sm px-4 py-1.5 rounded-lg transition"
				style="border:1px solid var(--c-border); color:var(--c-muted); background:transparent;"
			>{$t('common.cancel')}</button>
			<button
				onclick={saveEditToken}
				disabled={savingToken}
				class="text-sm font-semibold px-4 py-1.5 rounded-lg disabled:opacity-60 transition"
				style="background:#fbbf24; color:#1a1200;"
			>{savingToken ? $t('tokens.savingToken') : $t('tokens.saveToken')}</button>
		</div>
	</div>
</div>
{/if}

<!-- New Token Modal -->
{#if showAddToken}
<div class="fixed inset-0 z-50 flex items-center justify-center p-4" style="background:rgba(0,0,0,.6);">
	<div style="background:var(--c-surface); border:1px solid var(--c-border); border-radius:16px; width:100%; max-width:420px; box-shadow:0 16px 48px rgba(0,0,0,.5);">
		<div class="px-6 py-5" style="border-bottom:1px solid var(--c-border);">
			<h2 class="text-base font-semibold" style="color:var(--c-text);">{$t('tokens.addToken')}</h2>
		</div>
		<div class="px-6 py-5 space-y-4">
			<div>
				<label class="block text-xs font-semibold uppercase tracking-wide mb-1.5" style="color:var(--c-muted);">{$t('tokens.name')}</label>
				<input
					type="text"
					bind:value={newTokenName}
					placeholder={$t('tokens.namePlaceholder')}
					class="w-full px-3 py-2 text-sm rounded-lg outline-none"
					style="background:var(--c-bg); border:1px solid var(--c-border); color:var(--c-text);"
					onfocus={(e) => { (e.currentTarget as HTMLElement).style.borderColor='#fbbf24'; }}
					onblur={(e) => { (e.currentTarget as HTMLElement).style.borderColor='var(--c-border)'; }}
				/>
			</div>
			<div>
				<label class="block text-xs font-semibold uppercase tracking-wide mb-1.5" style="color:var(--c-muted);">
					{$t('tokens.allowedOrigins')} <span class="normal-case font-normal">{$t('tokens.originsHint')}</span>
				</label>
				<textarea
					bind:value={newTokenOrigins}
					rows="3"
					placeholder="https://myapp.com"
					class="w-full px-3 py-2 text-sm rounded-lg outline-none resize-none"
					style="background:var(--c-bg); border:1px solid var(--c-border); color:var(--c-text); font-family:'JetBrains Mono',monospace; font-size:12px;"
					onfocus={(e) => { (e.currentTarget as HTMLElement).style.borderColor='#fbbf24'; }}
					onblur={(e) => { (e.currentTarget as HTMLElement).style.borderColor='var(--c-border)'; }}
				></textarea>
			</div>
		</div>
		<div class="flex justify-end gap-2 px-6 py-4" style="border-top:1px solid var(--c-border);">
			<button
				onclick={() => { showAddToken = false; }}
				class="text-sm px-4 py-1.5 rounded-lg transition"
				style="border:1px solid var(--c-border); color:var(--c-muted); background:transparent;"
			>{$t('common.cancel')}</button>
			<button
				onclick={createToken}
				disabled={creatingToken}
				class="text-sm font-semibold px-4 py-1.5 rounded-lg disabled:opacity-60 transition"
				style="background:#fbbf24; color:#1a1200;"
			>{creatingToken ? $t('tokens.savingToken') : $t('tokens.saveToken')}</button>
		</div>
	</div>
</div>
{/if}

<!-- Token reveal Modal -->
{#if revealedToken}
<div class="fixed inset-0 z-50 flex items-center justify-center p-4" style="background:rgba(0,0,0,.6);">
	<div style="background:var(--c-surface); border:1px solid var(--c-border); border-radius:16px; width:100%; max-width:420px; box-shadow:0 16px 48px rgba(0,0,0,.5);">
		<div class="px-6 py-5" style="border-bottom:1px solid var(--c-border);">
			<h2 class="text-base font-semibold" style="color:var(--c-text);">{$t('apps.revealToken.title')}</h2>
			<p class="text-xs mt-1" style="color:#f87171;">{$t('apps.revealToken.hint')}</p>
		</div>
		<div class="px-6 py-5">
			<p class="text-xs font-semibold uppercase tracking-wide mb-2" style="color:var(--c-muted);">{revealedToken.name}</p>
			<code class="block w-full text-xs rounded-lg px-3 py-3 break-all" style="background:var(--c-bg); border:1px solid rgba(251,191,36,.3); color:#fbbf24; font-family:'JetBrains Mono',monospace;">{revealedToken.token}</code>
		</div>
		<div class="flex justify-end gap-2 px-6 py-4" style="border-top:1px solid var(--c-border);">
			<button
				onclick={() => { navigator.clipboard.writeText(revealedToken!.token); toast.success('Token copied.'); }}
				class="text-sm font-semibold px-4 py-1.5 rounded-lg transition"
				style="background:#fbbf24; color:#1a1200;"
			>{$t('apps.revealToken.copy')}</button>
			<button
				onclick={() => { revealedToken = null; }}
				class="text-sm px-4 py-1.5 rounded-lg transition"
				style="border:1px solid var(--c-border); color:var(--c-muted); background:transparent;"
			>{$t('apps.revealToken.close')}</button>
		</div>
	</div>
</div>
{/if}
