export function debounce(fn: () => void, ms = 280): () => void {
	let t: ReturnType<typeof setTimeout> | undefined;
	return () => {
		clearTimeout(t);
		t = setTimeout(fn, ms);
	};
}
