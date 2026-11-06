<script lang="ts">
	interface FwDevice {
		name: string;
		version: string;
	}
	let { devices }: { devices: FwDevice[] } = $props();

	// "0.9.1-dev.20261002094801+63126c4" -> "0.9.1 (dev build)".
	function short(v: string): string {
		const m = v.match(/^([\d.]+)(?:-(dev|beta|rc)[.\d]*)?/);
		return m ? `Version ${m[1]}${m[2] ? ` (${m[2]} build)` : ""}` : v;
	}
</script>

<div class="pkgs">
	<!-- fwupd's Name is a human label and can repeat across identical devices. -->
	{#each devices as device, i (`${device.name}:${i}`)}
		<div class="pkg"><span>{device.name}</span><small>{short(device.version)}</small></div>
	{/each}
</div>

<style>
	.pkgs { columns:2 220px; column-gap:16px; padding:2px 2px 10px; max-height:50vh; overflow:auto; }
	.pkg { break-inside:avoid; display:flex; justify-content:space-between; gap:8px; font-size:12.5px; padding:3px 0; }
	.pkg small { color:var(--faint); overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
</style>
