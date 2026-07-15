/// Minecraft skin face (with hat overlay) for a player UUID.
/// Uses minotar.net — reliable and returns a Steve fallback for unknown UUIDs
/// (crafatar has frequent outages). Request 2× the display size for crispness.
export function skinFace(uuid: string, displaySize: number): string {
  return `https://minotar.net/helm/${uuid}/${displaySize * 2}.png`;
}
