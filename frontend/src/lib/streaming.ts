// Frontend half of the event-streaming command pattern (see
// src-tauri/src/commands/streaming.rs) — runs a command, appends each
// stdout/stderr line to a reactive log as it arrives, and resolves once the
// process exits with whether it succeeded.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export async function runStreamingCommand(
	program: string,
	args: string[],
	onLine: (line: string) => void,
): Promise<boolean> {
	const sessionId = crypto.randomUUID();

	const unlisten = await listen<{ session_id: string; line: string }>("cmd-output", (event) => {
		if (event.payload.session_id === sessionId) onLine(event.payload.line);
	});

	try {
		return await invoke<boolean>("run_streaming_command", { sessionId, program, args });
	} finally {
		unlisten();
	}
}
