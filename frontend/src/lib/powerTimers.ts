/** Friendly values for the Power page's idle timers, in seconds. */
export const TIMER_PRESETS = [
	"0", // Never
	"60",
	"120",
	"300",
	"600",
	"900",
	"1200",
	"1800",
	"3600",
];

/** "0" -> Never, "470" -> 7 minutes 50 seconds. */
export function timerLabel(value: string): string {
	const seconds = Number(value);
	if (!Number.isFinite(seconds) || seconds <= 0) return "Never";
	const hours = Math.floor(seconds / 3600);
	const minutes = Math.floor((seconds % 3600) / 60);
	const rest = seconds % 60;
	const part = (n: number, unit: string) => `${n} ${unit}${n === 1 ? "" : "s"}`;
	return [hours && part(hours, "hour"), minutes && part(minutes, "minute"), rest && part(rest, "second")]
		.filter(Boolean)
		.join(" ");
}

/**
 * Presets plus the value currently in hypridle.conf. A value that is not one
 * of the presets keeps its literal seconds and is labelled as the current
 * custom value, so the dropdown neither hides nor silently rounds what the
 * config says. The caller keeps that value unless the user picks another.
 */
export function timerChoices(current: number | string): {
	options: string[];
	labels: Record<string, string>;
} {
	const value = String(current);
	const options = Array.from(new Set([...TIMER_PRESETS, value])).sort((a, b) => Number(a) - Number(b));
	const labels = Object.fromEntries(
		options.map((option) => [
			option,
			option === value && !TIMER_PRESETS.includes(option)
				? `Custom: ${timerLabel(option)} (current)`
				: timerLabel(option),
		]),
	);
	return { options, labels };
}
