import { skinCache } from "$lib/stores/skins.svelte";

export function skinFace(uuid: string, displaySize: number): string {
  return skinCache.getFace(uuid) ?? `https://minotar.net/helm/${uuid}/${displaySize * 2}.png`;
}
