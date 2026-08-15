/** Sidebar page switch. Set from +page.svelte; views call it to jump. */
export type Navigate = (page: string) => void;
export const NAVIGATE_KEY = "bos-settings-navigate";
