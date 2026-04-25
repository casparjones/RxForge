<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import { toast } from '$lib/stores/toast';
	import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';

	let apps = $state<any[]>([]);
	let loading = $state(true);
	let showCreateModal = $state(false);
	let newAppName = $state('');
	let newAppRedirects = $state('');
	let newAppAuthType = $state<'oauth' | 'token'>('oauth');
	let newAppDbScope = $state<'isolated' | 'shared'>('isolated');
	let creating = $state(false);

	// Expanded / edit state per app
	let expandedApp = $state<string | null>(null);
	let editName = $state<Record<string, string>>({});
	let editAuthType = $state<Record<string, string>>({});
	let editDbScope = $state<Record<string, string>>({});
	let editRedirects = $state<Record<string, string>>({});
	let saving = $state<Record<string, boolean>>({});

	// Credentials UI state
	let secretVisible = $state<Record<string, boolean>>({});
	let appStats = $state<Record<string, any>>({});

	// Token management state
	let appTokens = $state<Record<string, any[]>>({});
	let loadingTokens = $state<Record<string, boolean>>({});
	let showAddTokenModal = $state(false);
	let addTokenForApp = $state<string | null>(null);
	let newTokenName = $state('');
	let newTokenOrigins = $state('');
	let creatingToken = $state(false);
	let revealedToken = $state<{ token: string; name: string } | null>(null);

	// Confirm dialog
	let confirmOpen = $state(false);
	let confirmTitle = $state('');
	let confirmMessage = $state('');
	let confirmAction = $state<() => void>(() => {});

	// Claude prompt modal
	let promptModalOpen = $state(false);
	let promptText = $state('');

	function openConfirm(title: string, message: string, action: () => void) {
		confirmTitle = title;
		confirmMessage = message;
		confirmAction = action;
		confirmOpen = true;
	}

	function toggleExpand(app: any) {
		if (expandedApp === app.id) {
			expandedApp = null;
		} else {
			expandedApp = app.id;
			editName[app.id] = app.name;
			editAuthType[app.id] = app.auth_type ?? 'oauth';
			editDbScope[app.id] = app.db_scope ?? 'isolated';
			editRedirects[app.id] = (app.redirect_uris ?? []).join('\n');
			loadStats(app.id);
			if (app.auth_type === 'token') loadTokens(app.id);
		}
	}

	async function loadTokens(appId: string) {
		loadingTokens[appId] = true;
		try {
			appTokens[appId] = await api.apps.tokens.list(appId);
		} catch {
			appTokens[appId] = [];
		} finally {
			loadingTokens[appId] = false;
		}
	}

	function openAddToken(appId: string) {
		addTokenForApp = appId;
		newTokenName = '';
		newTokenOrigins = '';
		showAddTokenModal = true;
	}

	async function createToken() {
		if (!addTokenForApp) return;
		creatingToken = true;
		try {
			const origins = newTokenOrigins.split('\n').map(s => s.trim()).filter(Boolean);
			const res = await api.apps.tokens.create(addTokenForApp, {
				name: newTokenName.trim() || undefined,
				allowed_origins: origins,
			});
			appTokens[addTokenForApp] = [res, ...(appTokens[addTokenForApp] ?? [])];
			showAddTokenModal = false;
			revealedToken = { token: res.token, name: res.name };
		} catch (e: any) {
			toast.error('Fehler: ' + e.message);
		} finally {
			creatingToken = false;
		}
	}

	async function revokeToken(appId: string, tokenId: string) {
		try {
			await api.apps.tokens.revoke(appId, tokenId);
			appTokens[appId] = appTokens[appId].map(t =>
				t.id === tokenId ? { ...t, revoked: true } : t
			);
			toast.success('Token widerrufen.');
		} catch (e: any) {
			toast.error('Fehler: ' + e.message);
		}
	}

	async function saveApp(app: any) {
		const id = app.id;
		saving[id] = true;
		try {
			const redirectUris = (editRedirects[id] ?? '')
				.split('\n').map((s: string) => s.trim()).filter(Boolean);
			const updated = await api.apps.update(id, {
				name: editName[id],
				auth_type: editAuthType[id],
				db_scope: editDbScope[id],
				redirect_uris: redirectUris,
			});
			apps = apps.map(a => a.id === id ? { ...a, ...updated } : a);
			toast.success('App gespeichert.');
		} catch (e: any) {
			toast.error('Fehler beim Speichern: ' + e.message);
		} finally {
			saving[id] = false;
		}
	}

	async function loadApps() {
		loading = true;
		try {
			apps = await api.apps.list();
		} catch (e: any) {
			toast.error('Failed to load apps: ' + e.message);
		} finally {
			loading = false;
		}
	}

	async function loadStats(appId: string) {
		try {
			appStats[appId] = await api.apps.getStats(appId);
		} catch {
			appStats[appId] = null;
		}
	}

	async function createApp() {
		if (!newAppName.trim()) { toast.error('App name is required.'); return; }
		creating = true;
		try {
			const redirectUris = newAppRedirects.split('\n').map(s => s.trim()).filter(Boolean);
			const app = await api.apps.create({
				name: newAppName.trim(),
				redirect_uris: redirectUris,
				auth_type: newAppAuthType,
				db_scope: newAppDbScope,
			});
			apps = [...apps, app];
			showCreateModal = false;
			newAppName = '';
			newAppRedirects = '';
			newAppAuthType = 'oauth';
			newAppDbScope = 'isolated';
			toast.success('App created!');
		} catch (e: any) {
			toast.error('Failed to create app: ' + e.message);
		} finally {
			creating = false;
		}
	}

	async function deleteApp(id: string) {
		try {
			await api.apps.delete(id);
			apps = apps.filter(a => a.id !== id);
			expandedApp = null;
			toast.success('App deleted.');
		} catch (e: any) {
			toast.error('Failed to delete app: ' + e.message);
		}
	}

	async function regenerateSecret(id: string) {
		try {
			const res = await api.apps.regenerateSecret(id) as any;
			apps = apps.map(a => a.id === id ? { ...a, client_secret: res.client_secret } : a);
			toast.success('Secret regenerated!');
		} catch (e: any) {
			toast.error('Failed to regenerate secret: ' + e.message);
		}
	}

	function copyToClipboard(text: string, label: string) {
		navigator.clipboard.writeText(text).then(() => toast.success(`${label} copied!`));
	}

	function generatePrompt(app: any): string {
		const base = window.location.origin;
		const appId = app.id;
		const isToken = app.auth_type === 'token';

		const authSection = isToken ? `\
**Auth Type:** Public API Token (rxft_...)

**How authentication works:**
- For read access (pull/stream): use the token directly as a Bearer token.
- For write access (push): first exchange the token for a short-lived JWT (15 min):
  POST ${base}/api/v1/auth/token/exchange
  Body: { "token": "<your_rxft_token>" }
  → Returns: { "access_token": "...", "token_type": "Bearer", "expires_in": 900 }

Implement an auto-refresh: store the JWT and exchange a new one before it expires (< 2 min remaining).` : `\
**Auth Type:** OAuth 2.0 (Authorization Code Flow)

**Client ID:** ${app.client_id}
**Redirect URIs:** ${app.redirect_uris?.join(', ') || '(none configured)'}

**OAuth endpoints:**
- Authorize: POST ${base}/oauth/authorize
- Token:     POST ${base}/oauth/token  (grant_type: authorization_code)
- Revoke:    POST ${base}/oauth/revoke

After the OAuth flow, use the returned access_token as a Bearer token for all sync requests.`;

		return `\
I want to integrate RxDB with RxForge for local-first offline sync in my web app.

**My RxForge app:**
- Name: ${app.name}
- App ID: ${appId}
- RxForge base URL: ${base}

${authSection}

**Sync API endpoints (all require Authorization: Bearer <token>):**
- Pull:   GET  ${base}/api/v1/sync/${appId}/pull?checkpoint=<seq>&limit=100
          Response: { documents: [...], checkpoint: "..." }
- Push:   POST ${base}/api/v1/sync/${appId}/push
          Body: { "documents": [...] }
          Response: { written: N, conflicts: ["doc_id", ...] }
- Stream: GET  ${base}/api/v1/sync/${appId}/stream  (SSE, continuous _changes feed)

The documents follow CouchDB format (_id, _rev, _deleted).
Checkpoints are CouchDB sequence values (opaque strings, pass back as-is).

**Task:**
Please generate complete TypeScript code to:
1. Initialize RxDB with a sample \`todos\` collection (fields: id, text, done, updatedAt)
2. Implement a custom RxDB replication plugin for RxForge that:
   - Pulls changes from the RxForge pull endpoint using checkpoint-based pagination
   - Pushes local changes to the RxForge push endpoint in batches
   - Subscribes to the SSE stream for live updates
   - Handles conflicts (last-write-wins by updatedAt)
3. Wire up the ${isToken ? 'token exchange (auto-refresh JWT before expiry)' : 'OAuth access_token'} as the Bearer token in every request
4. Export a \`startSync(${isToken ? 'rxftToken' : 'accessToken'}: string)\` function that starts the replication

Use rxdb 15.x. Show the full working code.`;
	}

	function openPromptModal(app: any) {
		promptText = generatePrompt(app);
		promptModalOpen = true;
	}

	function copyPrompt() {
		navigator.clipboard.writeText(promptText).then(() => toast.success('Prompt copied!'));
	}

	onMount(loadApps);
</script>

<div class="space-y-6">
	<div class="flex items-center justify-between">
		<div>
			<div style="font-family:'JetBrains Mono',monospace; font-size:10px; letter-spacing:.15em; color:#7c7cff; margin-bottom:8px; text-transform:uppercase;">── Apps</div>
			<h1 class="text-2xl font-semibold" style="letter-spacing:-.02em; color:var(--c-text);">My Apps</h1>
		</div>
		<button
			onclick={() => { showCreateModal = true; }}
			class="px-4 py-2 rounded-lg text-sm font-semibold transition"
			style="background:#7c7cff; color:#05050f;"
			onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background='#9090ff'; }}
			onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background='#7c7cff'; }}
		>+ New App</button>
	</div>

	{#if loading}
		<div class="flex justify-center py-16">
			<div class="w-6 h-6 rounded-full border-2 border-t-transparent animate-spin" style="border-color:#7c7cff; border-top-color:transparent;"></div>
		</div>
	{:else if apps.length === 0}
		<div class="text-center py-16 rounded-2xl" style="background:var(--c-surface); border:1px solid var(--c-border);">
			<p class="mb-4" style="color:var(--c-muted);">No apps yet. Create your first app!</p>
			<button
				onclick={() => { showCreateModal = true; }}
				class="px-4 py-2 rounded-lg text-sm font-semibold transition"
				style="background:#7c7cff; color:#05050f;"
				onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background='#9090ff'; }}
				onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background='#7c7cff'; }}
			>Create App</button>
		</div>
	{:else}
		<div class="grid gap-4">
			{#each apps as app (app.id)}
				<div class="rounded-2xl overflow-hidden" style="background:var(--c-surface); border:1px solid var(--c-border);">

					<!-- App header row -->
					<div class="p-6">
						<div class="flex items-start justify-between">
							<div>
								<div class="flex items-center gap-2">
									<h2 class="text-lg font-semibold" style="color:var(--c-text);">{app.name}</h2>
									<span class="text-xs px-2 py-0.5 rounded-full font-medium" style="{app.auth_type === 'token' ? 'background:rgba(251,191,36,.12); color:#fbbf24;' : 'background:rgba(124,124,255,.12); color:#7c7cff;'}">
										{app.auth_type === 'token' ? 'Token' : 'OAuth'}
									</span>
									{#if app.db_scope === 'shared'}
										<span class="text-xs px-2 py-0.5 rounded-full font-medium" style="background:rgba(248,113,113,.12); color:#f87171;">Shared DB</span>
									{/if}
								</div>
								<p class="text-sm mt-0.5 font-mono" style="color:var(--c-muted);">ID: {app.id}</p>
							</div>
							<div class="flex items-center gap-2 flex-wrap justify-end">
								<button
									onclick={() => openPromptModal(app)}
									title="Claude-Prompt für RxDB-Integration generieren"
									class="text-sm font-medium px-3 py-1.5 rounded-lg transition flex items-center gap-1.5"
									style="color:#a78bfa; border:1px solid rgba(167,139,250,.25); background:rgba(167,139,250,.06);"
									onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background='rgba(167,139,250,.12)'; }}
									onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background='rgba(167,139,250,.06)'; }}
								>
									<svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="currentColor">
										<path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-2 14.5v-9l6 4.5-6 4.5z"/>
									</svg>
									Claude Prompt
								</button>
								<button
									onclick={() => toggleExpand(app)}
									class="text-sm font-medium transition"
									style="color:#7c7cff;"
									onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.color='#9090ff'; }}
									onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.color='#7c7cff'; }}
								>
									{expandedApp === app.id ? 'Schließen' : 'Bearbeiten'}
								</button>
							</div>
						</div>
					</div>

					<!-- Expanded edit + details panel -->
					{#if expandedApp === app.id}
						<div style="border-top:1px solid var(--c-border); background:var(--c-surface-2);">

							<!-- ── Editable settings ── -->
							<div class="px-6 pt-5 pb-4 space-y-4">
								<p class="text-xs font-semibold uppercase tracking-wide" style="color:var(--c-muted);">Einstellungen</p>

								<!-- Name -->
								<div>
									<label for="name-{app.id}" class="block text-xs font-medium mb-1 uppercase tracking-wide" style="color:var(--c-muted);">App Name</label>
									<input
										id="name-{app.id}"
										type="text"
										bind:value={editName[app.id]}
										class="w-full px-3 py-2 text-sm rounded-lg outline-none"
										style="background:var(--c-surface); border:1px solid var(--c-border); color:var(--c-text);"
										onfocus={(e) => { (e.currentTarget as HTMLElement).style.borderColor='#7c7cff'; }}
										onblur={(e) => { (e.currentTarget as HTMLElement).style.borderColor='var(--c-border)'; }}
									/>
								</div>

								<!-- Auth Type -->
								<div>
									<p class="block text-xs font-medium mb-2 uppercase tracking-wide" style="color:var(--c-muted);">Auth Type</p>
									<div class="grid grid-cols-2 gap-3">
										<label class="flex flex-col gap-1 rounded-lg p-3 cursor-pointer transition" style="{editAuthType[app.id] === 'oauth' ? 'border:1px solid #7c7cff; background:rgba(124,124,255,.06);' : 'border:1px solid var(--c-border); background:var(--c-surface);'}">
											<input type="radio" bind:group={editAuthType[app.id]} value="oauth" class="sr-only" />
											<span class="font-medium text-sm" style="color:var(--c-text);">OAuth 2.0</span>
											<span class="text-xs" style="color:var(--c-muted);">Authorization Code — für Apps mit Backend</span>
										</label>
										<label class="flex flex-col gap-1 rounded-lg p-3 cursor-pointer transition" style="{editAuthType[app.id] === 'token' ? 'border:1px solid #fbbf24; background:rgba(251,191,36,.06);' : 'border:1px solid var(--c-border); background:var(--c-surface);'}">
											<input type="radio" bind:group={editAuthType[app.id]} value="token" class="sr-only" />
											<span class="font-medium text-sm" style="color:var(--c-text);">Public Token</span>
											<span class="text-xs" style="color:var(--c-muted);">API-Key — auch für SPAs &amp; statische Seiten</span>
										</label>
									</div>
									{#if editAuthType[app.id] === 'token'}
										<p class="mt-2 text-xs rounded-lg px-3 py-2" style="color:#fbbf24; background:rgba(251,191,36,.06); border:1px solid rgba(251,191,36,.2);">
											⚠️ Der Token ist im JS-Code sichtbar. Aktiviere Origin-Binding beim Token, um Missbrauch einzuschränken.
										</p>
									{/if}
								</div>

								<!-- DB Scope -->
								<div>
									<p class="block text-xs font-medium mb-2 uppercase tracking-wide" style="color:var(--c-muted);">Datenbank-Scope</p>
									<div class="grid grid-cols-2 gap-3">
										<label class="flex flex-col gap-1 rounded-lg p-3 cursor-pointer transition" style="{editDbScope[app.id] === 'isolated' ? 'border:1px solid #7c7cff; background:rgba(124,124,255,.06);' : 'border:1px solid var(--c-border); background:var(--c-surface);'}">
											<input type="radio" bind:group={editDbScope[app.id]} value="isolated" class="sr-only" />
											<span class="font-medium text-sm" style="color:var(--c-text);">Isoliert</span>
											<span class="text-xs" style="color:var(--c-muted);">Jeder Nutzer hat seine eigene DB</span>
										</label>
										<label class="flex flex-col gap-1 rounded-lg p-3 cursor-pointer transition" style="{editDbScope[app.id] === 'shared' ? 'border:1px solid #f87171; background:rgba(248,113,113,.06);' : 'border:1px solid var(--c-border); background:var(--c-surface);'}">
											<input type="radio" bind:group={editDbScope[app.id]} value="shared" class="sr-only" />
											<span class="font-medium text-sm" style="color:var(--c-text);">Geteilt</span>
											<span class="text-xs" style="color:var(--c-muted);">Alle Nutzer teilen eine DB</span>
										</label>
									</div>
									{#if editDbScope[app.id] === 'shared'}
										<p class="mt-2 text-xs rounded-lg px-3 py-2" style="color:#f87171; background:rgba(248,113,113,.06); border:1px solid rgba(248,113,113,.2);">
											⚠️ Im geteilten Scope sehen und schreiben alle authentifizierten Nutzer dieselbe Datenbank. Deine App ist selbst für Zugriffskontrolle auf Dokumentebene verantwortlich (z.B. <code>owner_id</code> prüfen).
										</p>
									{/if}
								</div>

								<!-- Redirect URIs (nur OAuth) -->
								{#if editAuthType[app.id] === 'oauth'}
								<div>
									<label for="redirects-{app.id}" class="block text-xs font-medium mb-1 uppercase tracking-wide" style="color:var(--c-muted);">
										Redirect URIs <span class="normal-case font-normal">(eine pro Zeile)</span>
									</label>
									<textarea
										id="redirects-{app.id}"
										bind:value={editRedirects[app.id]}
										rows="3"
										class="w-full px-3 py-2 text-sm rounded-lg outline-none resize-none"
										style="background:var(--c-surface); border:1px solid var(--c-border); color:var(--c-text); font-family:'JetBrains Mono',monospace; font-size:12px;"
										placeholder="https://myapp.com/callback"
										onfocus={(e) => { (e.currentTarget as HTMLElement).style.borderColor='#7c7cff'; }}
										onblur={(e) => { (e.currentTarget as HTMLElement).style.borderColor='var(--c-border)'; }}
									></textarea>
								</div>
								{/if}

								<!-- Save / Delete row -->
								<div class="flex items-center justify-between pt-1">
									<button
										onclick={() => openConfirm(
											'App löschen',
											`"${app.name}" unwiderruflich löschen?`,
											() => { deleteApp(app.id); confirmOpen = false; }
										)}
										class="text-sm font-medium px-3 py-1.5 rounded-lg transition"
										style="color:#f87171; border:1px solid rgba(248,113,113,.25); background:transparent;"
										onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background='rgba(248,113,113,.08)'; }}
										onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background='transparent'; }}
									>App löschen</button>
									<button
										onclick={() => saveApp(app)}
										disabled={saving[app.id]}
										class="text-sm font-semibold px-4 py-1.5 rounded-lg disabled:opacity-60 transition"
										style="background:#7c7cff; color:#05050f;"
										onmouseenter={(e) => { if (!saving[app.id]) (e.currentTarget as HTMLElement).style.background='#9090ff'; }}
										onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background='#7c7cff'; }}
									>{saving[app.id] ? 'Speichern…' : 'Speichern'}</button>
								</div>
							</div>

							<!-- ── Credentials / Tokens ── -->
							{#if app.auth_type === 'token'}
							<!-- Token-App: Token-Verwaltung -->
							<div class="px-6 py-4 space-y-3" style="border-top:1px solid var(--c-border);">
								<div class="flex items-center justify-between">
									<p class="text-xs font-semibold uppercase tracking-wide" style="color:var(--c-muted);">Public Tokens</p>
									<button
										onclick={() => openAddToken(app.id)}
										class="text-xs font-semibold px-3 py-1.5 rounded-lg transition"
										style="background:#fbbf24; color:#1a1200;"
										onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background='#fcd34d'; }}
										onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background='#fbbf24'; }}
									>+ Token hinzufügen</button>
								</div>

								{#if loadingTokens[app.id]}
									<p class="text-sm" style="color:var(--c-muted);">Lädt…</p>
								{:else if !appTokens[app.id]?.length}
									<p class="text-sm py-2" style="color:var(--c-muted);">Noch kein Token. Erstelle einen um die App zu nutzen.</p>
								{:else}
									<div class="space-y-2">
										{#each appTokens[app.id] as tok (tok.id)}
											<div class="rounded-lg px-4 py-3 flex items-start justify-between gap-3" style="background:var(--c-surface); border:1px solid {tok.revoked ? 'var(--c-border)' : 'rgba(251,191,36,.2)'}; opacity:{tok.revoked ? .5 : 1};">
												<div class="min-w-0">
													<div class="flex items-center gap-2 flex-wrap">
														<code class="text-xs font-mono" style="color:#fbbf24;">{tok.token_prefix}…</code>
														<span class="text-xs font-medium" style="color:var(--c-text);">{tok.name}</span>
														{#if tok.revoked}
															<span class="text-xs px-1.5 py-0.5 rounded" style="background:rgba(248,113,113,.12); color:#f87171;">Widerrufen</span>
														{:else}
															<span class="text-xs px-1.5 py-0.5 rounded" style="background:rgba(74,222,128,.1); color:#4ade80;">Aktiv</span>
														{/if}
													</div>
													{#if tok.allowed_origins?.length}
														<p class="text-xs mt-1 font-mono truncate" style="color:var(--c-muted);">Origins: {tok.allowed_origins.join(', ')}</p>
													{:else}
														<p class="text-xs mt-1" style="color:var(--c-muted);">Alle Origins erlaubt</p>
													{/if}
													{#if tok.last_used_at}
														<p class="text-xs mt-0.5" style="color:var(--c-muted);">Zuletzt genutzt: {new Date(tok.last_used_at).toLocaleDateString('de')}</p>
													{/if}
												</div>
												{#if !tok.revoked}
													<button
														onclick={() => openConfirm('Token widerrufen', `"${tok.name}" unwiderruflich deaktivieren?`, () => { revokeToken(app.id, tok.id); confirmOpen = false; })}
														class="text-xs px-2 py-1 rounded shrink-0 transition"
														style="color:#f87171; border:1px solid rgba(248,113,113,.25); background:transparent;"
														onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background='rgba(248,113,113,.08)'; }}
														onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background='transparent'; }}
													>Widerrufen</button>
												{/if}
											</div>
										{/each}
									</div>
								{/if}
							</div>
							{:else}
							<!-- OAuth-App: Client ID + Secret -->
							<div class="px-6 py-4 space-y-4" style="border-top:1px solid var(--c-border);">
								<p class="text-xs font-semibold uppercase tracking-wide" style="color:var(--c-muted);">OAuth Credentials</p>

								<div>
									<p class="block text-xs font-medium mb-1 uppercase tracking-wide" style="color:var(--c-muted);">Client ID</p>
									<div class="flex items-center gap-2">
										<code class="flex-1 text-sm rounded px-3 py-1.5 font-mono" style="background:var(--c-surface); border:1px solid var(--c-border); color:var(--c-text);">{app.client_id}</code>
										<button onclick={() => copyToClipboard(app.client_id, 'Client ID')} class="text-xs rounded px-2 py-1.5 transition" style="color:#7c7cff; border:1px solid rgba(124,124,255,.25); background:transparent;" onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background='rgba(124,124,255,.08)'; }} onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background='transparent'; }}>Copy</button>
									</div>
								</div>

								<div>
									<p class="block text-xs font-medium mb-1 uppercase tracking-wide" style="color:var(--c-muted);">Client Secret</p>
									<div class="flex items-center gap-2">
										<code class="flex-1 text-sm rounded px-3 py-1.5 font-mono" style="background:var(--c-surface); border:1px solid var(--c-border); color:var(--c-text);">
											{secretVisible[app.id] ? app.client_secret : '••••••••••••••••'}
										</code>
										<button onclick={() => { secretVisible = { ...secretVisible, [app.id]: !secretVisible[app.id] }; }} class="text-xs rounded px-2 py-1.5 transition" style="color:var(--c-muted); border:1px solid var(--c-border); background:transparent;" onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.borderColor='var(--c-border-hi)'; }} onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.borderColor='var(--c-border)'; }}>
											{secretVisible[app.id] ? 'Hide' : 'Show'}
										</button>
										<button onclick={() => copyToClipboard(app.client_secret, 'Client Secret')} class="text-xs rounded px-2 py-1.5 transition" style="color:#7c7cff; border:1px solid rgba(124,124,255,.25); background:transparent;" onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background='rgba(124,124,255,.08)'; }} onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background='transparent'; }}>Copy</button>
										<button onclick={() => openConfirm('Secret neu generieren', 'Das aktuelle Secret wird ungültig. Fortfahren?', () => { regenerateSecret(app.id); confirmOpen = false; })} class="text-xs rounded px-2 py-1.5 transition" style="color:#fb923c; border:1px solid rgba(251,146,60,.25); background:transparent;" onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background='rgba(251,146,60,.08)'; }} onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background='transparent'; }}>Regenerate</button>
									</div>
								</div>
							</div>
							{/if}

							<!-- ── Stats ── -->
							<div class="px-6 py-4" style="border-top:1px solid var(--c-border);">
								<p class="text-xs font-semibold uppercase tracking-wide mb-3" style="color:var(--c-muted);">Statistik</p>
								{#if appStats[app.id] === undefined}
									<p class="text-sm" style="color:var(--c-muted);">Lädt…</p>
								{:else if appStats[app.id] === null}
									<p class="text-sm" style="color:var(--c-muted);">Keine Statistik verfügbar</p>
								{:else}
									<div class="grid grid-cols-3 gap-3">
										<div class="rounded-lg p-3 text-center" style="background:var(--c-surface); border:1px solid var(--c-border);">
											<p class="text-2xl font-bold" style="color:#7c7cff;">{appStats[app.id].requests_today ?? 0}</p>
											<p class="text-xs mt-0.5" style="color:var(--c-muted);">Heute</p>
										</div>
										<div class="rounded-lg p-3 text-center" style="background:var(--c-surface); border:1px solid var(--c-border);">
											<p class="text-2xl font-bold" style="color:#7c7cff;">{appStats[app.id].requests_7d ?? 0}</p>
											<p class="text-xs mt-0.5" style="color:var(--c-muted);">7 Tage</p>
										</div>
										<div class="rounded-lg p-3 text-center" style="background:var(--c-surface); border:1px solid var(--c-border);">
											<p class="text-2xl font-bold" style="color:#7c7cff;">{appStats[app.id].requests_30d ?? 0}</p>
											<p class="text-xs mt-0.5" style="color:var(--c-muted);">30 Tage</p>
										</div>
									</div>
								{/if}
							</div>
						</div>
					{/if}
				</div>
			{/each}
		</div>
	{/if}
</div>

<!-- Create App Modal -->
{#if showCreateModal}
	<div class="fixed inset-0 z-40 flex items-center justify-center p-4" style="background:rgba(0,0,0,.6);" onclick={() => { showCreateModal = false; }} role="presentation">
		<div class="rounded-2xl shadow-2xl max-w-md w-full p-6" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()} role="dialog" tabindex="-1" aria-modal="true" style="background:var(--c-surface); border:1px solid var(--c-border);">
			<h2 class="text-xl font-semibold mb-5" style="color:var(--c-text);">Neue App erstellen</h2>
			<form onsubmit={(e) => { e.preventDefault(); createApp(); }} class="space-y-4">
				<div>
					<label for="appName" class="block text-sm font-medium mb-1" style="color:var(--c-text);">App Name</label>
					<input id="appName" type="text" bind:value={newAppName} required class="w-full px-4 py-2.5 rounded-lg outline-none" style="background:var(--c-surface-2); border:1px solid var(--c-border); color:var(--c-text);" placeholder="My App"
						onfocus={(e) => { (e.currentTarget as HTMLElement).style.borderColor='#7c7cff'; }}
						onblur={(e) => { (e.currentTarget as HTMLElement).style.borderColor='var(--c-border)'; }}
					/>
				</div>

				<div>
					<p class="block text-sm font-medium mb-2" style="color:var(--c-text);">Auth Type</p>
					<div class="grid grid-cols-2 gap-3">
						<label class="flex flex-col gap-1 rounded-lg p-3 cursor-pointer transition" style="{newAppAuthType === 'oauth' ? 'border:1px solid #7c7cff; background:rgba(124,124,255,.06);' : 'border:1px solid var(--c-border); background:var(--c-surface-2);'}">
							<input type="radio" bind:group={newAppAuthType} value="oauth" class="sr-only" />
							<span class="font-medium text-sm" style="color:var(--c-text);">OAuth 2.0</span>
							<span class="text-xs" style="color:var(--c-muted);">Authorization Code — für Apps mit eigenem Backend</span>
						</label>
						<label class="flex flex-col gap-1 rounded-lg p-3 cursor-pointer transition" style="{newAppAuthType === 'token' ? 'border:1px solid #fbbf24; background:rgba(251,191,36,.06);' : 'border:1px solid var(--c-border); background:var(--c-surface-2);'}">
							<input type="radio" bind:group={newAppAuthType} value="token" class="sr-only" />
							<span class="font-medium text-sm" style="color:var(--c-text);">Public Token</span>
							<span class="text-xs" style="color:var(--c-muted);">API-Key — auch für SPAs &amp; statische Seiten</span>
						</label>
					</div>
					{#if newAppAuthType === 'token'}
						<p class="mt-2 text-xs rounded-lg px-3 py-2" style="color:#fbbf24; background:rgba(251,191,36,.06); border:1px solid rgba(251,191,36,.2);">
							⚠️ Der Token ist im JS-Code sichtbar. Aktiviere Origin-Binding beim Token, um Missbrauch einzuschränken.
						</p>
					{/if}
				</div>

				<div>
					<p class="block text-sm font-medium mb-2" style="color:var(--c-text);">Datenbank-Scope</p>
					<div class="grid grid-cols-2 gap-3">
						<label class="flex flex-col gap-1 rounded-lg p-3 cursor-pointer transition" style="{newAppDbScope === 'isolated' ? 'border:1px solid #7c7cff; background:rgba(124,124,255,.06);' : 'border:1px solid var(--c-border); background:var(--c-surface-2);'}">
							<input type="radio" bind:group={newAppDbScope} value="isolated" class="sr-only" />
							<span class="font-medium text-sm" style="color:var(--c-text);">Isoliert</span>
							<span class="text-xs" style="color:var(--c-muted);">Jeder Nutzer hat seine eigene DB</span>
						</label>
						<label class="flex flex-col gap-1 rounded-lg p-3 cursor-pointer transition" style="{newAppDbScope === 'shared' ? 'border:1px solid #f87171; background:rgba(248,113,113,.06);' : 'border:1px solid var(--c-border); background:var(--c-surface-2);'}">
							<input type="radio" bind:group={newAppDbScope} value="shared" class="sr-only" />
							<span class="font-medium text-sm" style="color:var(--c-text);">Geteilt</span>
							<span class="text-xs" style="color:var(--c-muted);">Alle Nutzer teilen eine DB</span>
						</label>
					</div>
					{#if newAppDbScope === 'shared'}
						<p class="mt-2 text-xs rounded-lg px-3 py-2" style="color:#f87171; background:rgba(248,113,113,.06); border:1px solid rgba(248,113,113,.2);">
							⚠️ Im geteilten Scope sehen und schreiben alle authentifizierten Nutzer dieselbe Datenbank. Deine App ist selbst für Zugriffskontrolle auf Dokumentebene verantwortlich.
						</p>
					{/if}
				</div>

				{#if newAppAuthType === 'oauth'}
				<div>
					<label for="redirectUris" class="block text-sm font-medium mb-1" style="color:var(--c-text);">
						Redirect URIs <span style="color:var(--c-muted); font-weight:normal;">(eine pro Zeile)</span>
					</label>
					<textarea id="redirectUris" bind:value={newAppRedirects} rows="3" class="w-full px-4 py-2.5 rounded-lg outline-none resize-none" style="background:var(--c-surface-2); border:1px solid var(--c-border); color:var(--c-text); font-family:'JetBrains Mono',monospace; font-size:12px;" placeholder="https://myapp.com/callback"
						onfocus={(e) => { (e.currentTarget as HTMLElement).style.borderColor='#7c7cff'; }}
						onblur={(e) => { (e.currentTarget as HTMLElement).style.borderColor='var(--c-border)'; }}
					></textarea>
				</div>
				{/if}

				<div class="flex gap-3 justify-end pt-2">
					<button type="button" onclick={() => { showCreateModal = false; }} class="px-4 py-2 text-sm font-medium rounded-lg transition" style="border:1px solid var(--c-border); color:var(--c-muted); background:transparent;"
						onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.borderColor='var(--c-border-hi)'; (e.currentTarget as HTMLElement).style.color='var(--c-text)'; }}
						onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.borderColor='var(--c-border)'; (e.currentTarget as HTMLElement).style.color='var(--c-muted)'; }}
					>Abbrechen</button>
					<button type="submit" disabled={creating} class="px-4 py-2 text-sm font-semibold disabled:opacity-60 rounded-lg transition" style="background:#7c7cff; color:#05050f;"
						onmouseenter={(e) => { if (!creating) (e.currentTarget as HTMLElement).style.background='#9090ff'; }}
						onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background='#7c7cff'; }}
					>{creating ? 'Erstelle…' : 'App erstellen'}</button>
				</div>
			</form>
		</div>
	</div>
{/if}

<!-- Claude Prompt Modal -->
{#if promptModalOpen}
	<div class="fixed inset-0 z-40 flex items-center justify-center p-4" style="background:rgba(0,0,0,.6);" onclick={() => { promptModalOpen = false; }} role="presentation">
		<div class="rounded-2xl shadow-2xl max-w-2xl w-full flex flex-col max-h-[85vh]" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()} role="dialog" tabindex="-1" aria-modal="true" style="background:var(--c-surface); border:1px solid var(--c-border);">
			<div class="px-6 py-4 flex items-center justify-between shrink-0" style="border-bottom:1px solid var(--c-border);">
				<div>
					<h2 class="text-lg font-semibold" style="color:var(--c-text);">Claude Integration Prompt</h2>
					<p class="text-sm mt-0.5" style="color:var(--c-muted);">In Claude einfügen, um RxDB-Integrationscode zu erhalten.</p>
				</div>
				<button onclick={() => { promptModalOpen = false; }} class="p-1 rounded-lg" style="color:var(--c-muted);" aria-label="Schließen">
					<svg class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12"/></svg>
				</button>
			</div>
			<div class="flex-1 overflow-auto px-6 py-4">
				<textarea readonly value={promptText} class="w-full h-full min-h-64 text-sm font-mono rounded-lg p-4 resize-none outline-none leading-relaxed" style="background:var(--c-surface-2); border:1px solid var(--c-border); color:var(--c-text);" onclick={(e) => (e.target as HTMLTextAreaElement).select()}></textarea>
			</div>
			<div class="px-6 py-4 flex items-center justify-between shrink-0" style="border-top:1px solid var(--c-border);">
				<p class="text-xs" style="color:var(--c-muted);">Klicken zum Auswählen</p>
				<div class="flex gap-2">
					<button onclick={() => { promptModalOpen = false; }} class="px-4 py-2 text-sm font-medium rounded-lg transition" style="border:1px solid var(--c-border); color:var(--c-muted); background:transparent;"
						onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.borderColor='var(--c-border-hi)'; }}
						onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.borderColor='var(--c-border)'; }}
					>Schließen</button>
					<button onclick={copyPrompt} class="px-4 py-2 text-sm font-medium rounded-lg transition flex items-center gap-2" style="background:#a78bfa; color:#fff;"
						onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background='#c4b5fd'; }}
						onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background='#a78bfa'; }}
					>
						<svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"/></svg>
						Prompt kopieren
					</button>
				</div>
			</div>
		</div>
	</div>
{/if}

<!-- Add Token Modal -->
{#if showAddTokenModal && addTokenForApp}
	<div class="fixed inset-0 z-40 flex items-center justify-center p-4" style="background:rgba(0,0,0,.6);" onclick={() => { showAddTokenModal = false; }} role="presentation">
		<div class="rounded-2xl shadow-2xl max-w-md w-full p-6" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()} role="dialog" tabindex="-1" aria-modal="true" style="background:var(--c-surface); border:1px solid var(--c-border);">
			<h2 class="text-xl font-semibold mb-5" style="color:var(--c-text);">Token hinzufügen</h2>
			<form onsubmit={(e) => { e.preventDefault(); createToken(); }} class="space-y-4">
				<div>
					<label for="tokenName" class="block text-sm font-medium mb-1" style="color:var(--c-text);">
						Name <span style="color:var(--c-muted); font-weight:normal;">(optional)</span>
					</label>
					<input
						id="tokenName"
						type="text"
						bind:value={newTokenName}
						class="w-full px-4 py-2.5 rounded-lg outline-none"
						style="background:var(--c-surface-2); border:1px solid var(--c-border); color:var(--c-text);"
						placeholder="z.B. Produktion, Dev, …"
						onfocus={(e) => { (e.currentTarget as HTMLElement).style.borderColor='#fbbf24'; }}
						onblur={(e) => { (e.currentTarget as HTMLElement).style.borderColor='var(--c-border)'; }}
					/>
				</div>

				<div>
					<label for="tokenOrigins" class="block text-sm font-medium mb-1" style="color:var(--c-text);">
						Erlaubte Origins <span style="color:var(--c-muted); font-weight:normal;">(eine pro Zeile, leer = alle)</span>
					</label>
					<textarea
						id="tokenOrigins"
						bind:value={newTokenOrigins}
						rows="3"
						class="w-full px-4 py-2.5 rounded-lg outline-none resize-none"
						style="background:var(--c-surface-2); border:1px solid var(--c-border); color:var(--c-text); font-family:'JetBrains Mono',monospace; font-size:12px;"
						placeholder="https://myapp.com&#10;https://dev.myapp.com"
						onfocus={(e) => { (e.currentTarget as HTMLElement).style.borderColor='#fbbf24'; }}
						onblur={(e) => { (e.currentTarget as HTMLElement).style.borderColor='var(--c-border)'; }}
					></textarea>
					<p class="text-xs mt-1" style="color:var(--c-muted);">Origin-Binding verhindert Missbrauch, wenn der Token öffentlich sichtbar ist.</p>
				</div>

				<div class="flex gap-3 justify-end pt-2">
					<button
						type="button"
						onclick={() => { showAddTokenModal = false; }}
						class="px-4 py-2 text-sm font-medium rounded-lg transition"
						style="border:1px solid var(--c-border); color:var(--c-muted); background:transparent;"
						onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.borderColor='var(--c-border-hi)'; (e.currentTarget as HTMLElement).style.color='var(--c-text)'; }}
						onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.borderColor='var(--c-border)'; (e.currentTarget as HTMLElement).style.color='var(--c-muted)'; }}
					>Abbrechen</button>
					<button
						type="submit"
						disabled={creatingToken}
						class="px-4 py-2 text-sm font-semibold disabled:opacity-60 rounded-lg transition"
						style="background:#fbbf24; color:#1a1200;"
						onmouseenter={(e) => { if (!creatingToken) (e.currentTarget as HTMLElement).style.background='#fcd34d'; }}
						onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background='#fbbf24'; }}
					>{creatingToken ? 'Erstelle…' : 'Token erstellen'}</button>
				</div>
			</form>
		</div>
	</div>
{/if}

<!-- Revealed Token Modal (one-time display) -->
{#if revealedToken}
	<div class="fixed inset-0 z-50 flex items-center justify-center p-4" style="background:rgba(0,0,0,.7);" role="presentation">
		<div class="rounded-2xl shadow-2xl max-w-lg w-full p-6" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()} role="dialog" tabindex="-1" aria-modal="true" style="background:var(--c-surface); border:1px solid rgba(251,191,36,.4);">
			<div class="flex items-center gap-3 mb-4">
				<div class="w-8 h-8 rounded-full flex items-center justify-center shrink-0" style="background:rgba(251,191,36,.15);">
					<svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="#fbbf24" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M12 9v2m0 4h.01M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z"/></svg>
				</div>
				<div>
					<h2 class="text-lg font-semibold" style="color:var(--c-text);">Token wurde erstellt</h2>
					<p class="text-sm" style="color:#fbbf24;">Wird nur jetzt einmal angezeigt — bitte sofort kopieren!</p>
				</div>
			</div>

			<p class="text-sm mb-2" style="color:var(--c-muted);">Token: <span class="font-medium" style="color:var(--c-text);">{revealedToken.name || '(ohne Name)'}</span></p>

			<div class="rounded-lg p-4 mb-4 flex items-center gap-3" style="background:var(--c-surface-2); border:1px solid rgba(251,191,36,.3);">
				<code class="flex-1 text-sm font-mono break-all select-all" style="color:#fbbf24;">{revealedToken.token}</code>
				<button
					onclick={() => { navigator.clipboard.writeText(revealedToken!.token).then(() => toast.success('Token kopiert!')); }}
					class="shrink-0 px-3 py-1.5 text-xs font-semibold rounded-lg transition"
					style="background:#fbbf24; color:#1a1200;"
					onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background='#fcd34d'; }}
					onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background='#fbbf24'; }}
				>Kopieren</button>
			</div>

			<p class="text-xs mb-5 rounded-lg px-3 py-2" style="color:#f87171; background:rgba(248,113,113,.06); border:1px solid rgba(248,113,113,.2);">
				Dieser Token wird nicht erneut angezeigt. Falls du ihn verlierst, musst du einen neuen erstellen und den alten widerrufen.
			</p>

			<div class="flex justify-end">
				<button
					onclick={() => { revealedToken = null; }}
					class="px-5 py-2 text-sm font-semibold rounded-lg transition"
					style="background:#7c7cff; color:#05050f;"
					onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background='#9090ff'; }}
					onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background='#7c7cff'; }}
				>Verstanden, Token gesichert</button>
			</div>
		</div>
	</div>
{/if}

<ConfirmDialog
	open={confirmOpen}
	title={confirmTitle}
	message={confirmMessage}
	confirmLabel="Bestätigen"
	destructive={true}
	onConfirm={confirmAction}
	onCancel={() => { confirmOpen = false; }}
/>
