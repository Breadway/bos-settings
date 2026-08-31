// Bridges bread-theme's pywal-derived palette into the webview. Mirrors
// bread_theme::gtk::apply_shared()'s two-phase pattern: fetch once at
// startup, then keep it live via a backend-pushed event  -  see
// src-tauri/src/commands/theme.rs for the file-watch side of this.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

const STYLE_ELEMENT_ID = "bread-theme";

function applyThemeCss(css: string) {
	let style = document.getElementById(STYLE_ELEMENT_ID);
	if (!style) {
		style = document.createElement("style");
		style.id = STYLE_ELEMENT_ID;
		document.head.appendChild(style);
	}
	style.textContent = css;
}

/** Call once at startup (e.g. from a root `$effect`/`onMount`). */
export async function initTheme(): Promise<void> {
	const css = await invoke<string>("get_theme_css");
	applyThemeCss(css);

	await listen<string>("theme-changed", (event) => {
		applyThemeCss(event.payload);
	});
}
