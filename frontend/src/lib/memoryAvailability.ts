export type MemoryAvailability = "checking" | "not-installed" | "available";

/** Three states, so a slow probe never flashes controls that cannot work. */
export function memoryAvailability(installed: boolean | null): MemoryAvailability {
	if (installed === null) return "checking";
	return installed ? "available" : "not-installed";
}
