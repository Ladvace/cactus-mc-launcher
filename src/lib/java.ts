/** The Java major version a Minecraft (release) version needs. Mirrors Mojang's
 *  runtime split: ≤1.16 → 8, 1.17–1.20.4 → 17, 1.20.5+ → 21. Snapshots/unknown
 *  fall back to the newest. Used for labelling; launch reads the exact value
 *  from the version manifest. */
export function requiredJavaMajor(mcVersion: string): number {
  const match = mcVersion.match(/^1\.(\d+)(?:\.(\d+))?/);
  if (!match) return 21;
  const minor = Number(match[1]);
  const patch = match[2] ? Number(match[2]) : 0;
  if (minor <= 16) return 8;
  if (minor < 20) return 17;
  if (minor === 20) return patch >= 5 ? 21 : 17;
  return 21;
}
