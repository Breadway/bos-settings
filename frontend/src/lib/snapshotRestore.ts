export function snapshotRestoreConfirmation(number: string): string {
	return `Your computer restarts into the system as it was at snapshot #${number}. The current system is kept as a backup, so this can be undone. Your files in Home are not changed.`;
}

export async function confirmAndRestoreSnapshot(
	number: string | null,
	confirmAction: (message: string) => boolean,
	restore: (number: string) => Promise<unknown>,
	onError: (message: string) => void,
): Promise<void> {
	if (!number || !confirmAction(snapshotRestoreConfirmation(number))) return;
	try {
		await restore(number);
	} catch (error) {
		onError(`${error}`);
	}
}
