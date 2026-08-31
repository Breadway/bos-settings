// Frontend half of the event-streaming command pattern (see
// src/src/commands/streaming.rs)  -  listens for `cmd-output` lines from a
// typed Tauri command that runs a hardcoded program, then resolves once
// the process exits.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export async function runStreamed(
	command: string,
	args: Record<string, unknown>,
	onLine: (line: string) => void,
): Promise<boolean> {
	const sessionId = crypto.randomUUID();

	const unlisten = await listen<{ session_id: string; line: string }>("cmd-output", (event) => {
		if (event.payload.session_id === sessionId) onLine(event.payload.line);
	});

	try {
		return await invoke<boolean>(command, { sessionId, ...args });
	} finally {
		unlisten();
	}
}
