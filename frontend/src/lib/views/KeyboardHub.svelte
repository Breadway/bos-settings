<script lang="ts">
 import Hub from "$lib/components/Hub.svelte";
 import KeyboardLayout from "./KeyboardLayout.svelte";
 import Keybinds from "./Keybinds.svelte";
 import InputMethod from "./InputMethod.svelte";
</script>
<Hub title="Keyboard" lede="Layouts, shortcuts and input methods." tabs={[
 { id: "layout", label: "Layout", component: KeyboardLayout },
 { id: "keybinds", label: "Shortcuts", component: Keybinds },
 { id: "ime", label: "Input method", component: InputMethod },
]} />
