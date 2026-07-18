import { toast } from "$lib/stores/toast.svelte";

/** Copy text to the clipboard and confirm with a toast. Clipboard failures are
 *  swallowed (nothing to do). */
export async function copyText(text: string, message = "Copied."): Promise<void> {
  try {
    await navigator.clipboard.writeText(text);
    toast.success(message);
  } catch {
    /* clipboard unavailable */
  }
}
