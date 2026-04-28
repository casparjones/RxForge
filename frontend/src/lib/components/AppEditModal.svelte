<script lang="ts">
	import { api } from '$lib/api';
	import { toast } from '$lib/stores/toast';
	import { get } from 'svelte/store';
	import { t } from '$lib/i18n';
	import ConfirmDialog from './ConfirmDialog.svelte';

	interface Props {
		app: any;
		onclose: () => void;
		onsaved: (updated: any) => void;
		ondeleted: () => void;
	}

	let { app, onclose, onsaved, ondeleted }: Props = $props();

	let editName = $state(app.name);
	let editAuthType = $state(app.auth_type ?? 'oauth');
	let editDbScope = $state(app.db_scope ?? 'isolated');
	let editRedirects = $state((app.redirect_uris ?? []).join('\n'));
	let saving = $state(false);

	let confirmOpen = $state(false);

	async function save() {
		saving = true;
		try {
			const redirectUris = editRedirects.split('\n').map((s: string) => s.trim()).filter(Boolean);
			const updated = await api.apps.update(app.id, {
				name: editName,
				auth_type: editAuthType,
				db_scope: editDbScope,
				redirect_uris: redirectUris,
			});
			onsaved({ ...app, ...updated });
			toast.success(get(t)('apps.appSaved'));
			onclose();
		} catch (e: any) {
			toast.error('Error: ' + e.message);
		} finally {
			saving = false;
		}
	}

	async function doDelete() {
		try {
			await api.apps.delete(app.id);
			ondeleted();
			toast.success(get(t)('apps.appDeleted'));
			onclose();
		} catch (e: any) {
			toast.error('Error: ' + e.message);
		}
	}
</script>

<!-- Backdrop -->
<div
	class="fixed inset-0 z-40 flex items-center justify-center p-4"
	style="background:rgba(0,0,0,.55);"
	onclick={onclose}
	role="presentation"
>
	<!-- Modal -->
	<div
		onclick={(e) => e.stopPropagation()}
		onkeydown={(e) => e.stopPropagation()}
		role="dialog"
		tabindex="-1"
		aria-modal="true"
		style="background:var(--c-surface); border:1px solid var(--c-border); border-radius:16px; width:100%; max-width:540px; max-height:90vh; overflow-y:auto; box-shadow:0 24px 64px rgba(0,0,0,.5);"
	>
		<!-- Header -->
		<div class="flex items-center justify-between px-6 py-4" style="border-bottom:1px solid var(--c-border);">
			<h2 style="font-size:16px; font-weight:600; color:var(--c-text);">{$t('apps.editTitle', { name: app.name })}</h2>
			<button onclick={onclose} style="color:var(--c-muted); font-size:20px; line-height:1; background:none; border:none; cursor:pointer;">×</button>
		</div>

		<!-- Body -->
		<div class="px-6 py-5 space-y-5">

			<!-- Name -->
			<div>
				<label class="block text-xs font-semibold uppercase tracking-wide mb-1.5" style="color:var(--c-muted);">{$t('apps.appName')}</label>
				<input
					type="text"
					bind:value={editName}
					class="w-full px-3 py-2 text-sm rounded-lg outline-none"
					style="background:var(--c-surface-2,var(--c-surface)); border:1px solid var(--c-border); color:var(--c-text);"
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
						⚠️ {$t('apps.authTokenWarning')}
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
						⚠️ {$t('apps.dbSharedWarning')}
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
						style="background:var(--c-surface-2,var(--c-surface)); border:1px solid var(--c-border); color:var(--c-text); font-family:'JetBrains Mono',monospace; font-size:12px;"
						placeholder="https://myapp.com/callback"
						onfocus={(e) => { (e.currentTarget as HTMLElement).style.borderColor='#7c7cff'; }}
						onblur={(e) => { (e.currentTarget as HTMLElement).style.borderColor='var(--c-border)'; }}
					></textarea>
				</div>
			{/if}
		</div>

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
					onclick={onclose}
					class="text-sm font-medium px-4 py-1.5 rounded-lg transition"
					style="border:1px solid var(--c-border); color:var(--c-muted); background:transparent;"
					onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.color='var(--c-text)'; }}
					onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.color='var(--c-muted)'; }}
				>{$t('common.cancel')}</button>
				<button
					onclick={save}
					disabled={saving}
					class="text-sm font-semibold px-4 py-1.5 rounded-lg disabled:opacity-60 transition"
					style="background:#7c7cff; color:#05050f;"
					onmouseenter={(e) => { if (!saving) (e.currentTarget as HTMLElement).style.background='#9090ff'; }}
					onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background='#7c7cff'; }}
				>{saving ? $t('common.saving') : $t('common.save')}</button>
			</div>
		</div>
	</div>
</div>

<ConfirmDialog
	open={confirmOpen}
	title={$t('apps.deleteApp')}
	message={$t('apps.deleteAppConfirm', { name: app.name })}
	confirmLabel={$t('common.delete')}
	destructive={true}
	onConfirm={() => { confirmOpen = false; doDelete(); }}
	onCancel={() => { confirmOpen = false; }}
/>
