import { toast } from "$lib/stores/toast.svelte";

export async function copyText(text: string, message = "Copied."): Promise<void> {
  try {
    await navigator.clipboard.writeText(text);
    toast.success(message);
  } catch {
  }
}
