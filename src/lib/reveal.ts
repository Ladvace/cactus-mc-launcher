import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { showError } from "$lib/stores/toast.svelte";

export async function revealPath(path: string, opts: { silent?: boolean } = {}): Promise<void> {
  try {
    await revealItemInDir(path);
  } catch (error) {
    if (!opts.silent) showError(error);
  }
}
