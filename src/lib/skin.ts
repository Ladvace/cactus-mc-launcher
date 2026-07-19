/// Uses minotar.net (Steve fallback for unknown UUIDs; crafatar has frequent outages).
export function skinFace(uuid: string, displaySize: number): string {
  return `https://minotar.net/helm/${uuid}/${displaySize * 2}.png`;
}
