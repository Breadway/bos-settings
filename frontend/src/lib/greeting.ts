/** Time-of-day greeting for the Home page. */
export function greetingFor(hour: number): string {
	if (hour < 5) return "Still up";
	if (hour < 12) return "Good morning";
	if (hour < 18) return "Good afternoon";
	return "Good evening";
}

/**
 * Add a person's name without producing "Still up?, liveuser". The late-night
 * greeting is a question, so the name goes before the question mark. An empty
 * or whitespace-only name leaves the plain greeting.
 */
export function greetingWithName(hour: number, name?: string | null): string {
	const base = greetingFor(hour);
	const person = typeof name === "string" ? name.trim() : "";
	if (!person) return hour < 5 ? `${base}?` : base;
	return hour < 5 ? `${base}, ${person}?` : `${base}, ${person}`;
}
