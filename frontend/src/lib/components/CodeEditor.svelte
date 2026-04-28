<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { EditorView, basicSetup } from 'codemirror';
	import { json } from '@codemirror/lang-json';
	import { oneDark } from '@codemirror/theme-one-dark';
	import { EditorState } from '@codemirror/state';

	interface Props {
		value: string;
		readonly?: boolean;
		onchange?: (val: string) => void;
		minHeight?: string;
	}

	let { value, readonly = false, onchange, minHeight = '200px' }: Props = $props();

	let container: HTMLDivElement;
	let view: EditorView | null = null;

	onMount(() => {
		view = new EditorView({
			state: EditorState.create({
				doc: value,
				extensions: [
					basicSetup,
					json(),
					oneDark,
					EditorView.editable.of(!readonly),
					EditorView.theme({
						'&': { fontSize: '13px', fontFamily: "'JetBrains Mono', monospace" },
						'.cm-scroller': { minHeight },
					}),
					EditorView.updateListener.of((update) => {
						if (update.docChanged) {
							onchange?.(update.state.doc.toString());
						}
					}),
				],
			}),
			parent: container,
		});
	});

	// Sync external value changes (e.g. loading a new document)
	$effect(() => {
		if (view && view.state.doc.toString() !== value) {
			view.dispatch({
				changes: { from: 0, to: view.state.doc.length, insert: value },
			});
		}
	});

	onDestroy(() => {
		view?.destroy();
		view = null;
	});
</script>

<div bind:this={container} style="border:1px solid var(--c-border); border-radius:8px; overflow:hidden;"></div>
