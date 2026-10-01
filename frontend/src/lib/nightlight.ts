// Night light warmth <-> colour temperature. The slider is labelled
// "warmth", so moving it right must make the screen warmer, which is a LOWER
// Kelvin value (BSET-29: the old slider ran in Kelvin, so dragging toward
// "more warm" produced 6100K, the coolest end).

export const MIN_KELVIN = 2000;
export const MAX_KELVIN = 6500;

/** 0 = least warm (6500K, no tint) .. 100 = warmest (2000K). */
export function kelvinToWarmth(kelvin: number): number {
	const k = Math.min(MAX_KELVIN, Math.max(MIN_KELVIN, kelvin));
	return Math.round(((MAX_KELVIN - k) / (MAX_KELVIN - MIN_KELVIN)) * 100);
}

/** Inverse of {@link kelvinToWarmth}, rounded to the nearest 100K. */
export function warmthToKelvin(warmth: number): number {
	const w = Math.min(100, Math.max(0, warmth));
	const k = MAX_KELVIN - (w / 100) * (MAX_KELVIN - MIN_KELVIN);
	return Math.round(k / 100) * 100;
}

/** Track gradient for the warmth slider: cool on the left, warm on the right. */
export const WARM_GRADIENT = "linear-gradient(90deg, #bcd7ff, #fff4e0 40%, #ffc27a 75%, #ff8a3d)";
