export function snapshotRestoreConfirmation(number: string): string {
	return `Make snapshot #${number} the permanent @ root? The current @ root will be preserved as a backup. The system will reboot automatically after the restore.`;
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
