// Curated quick-connect list of well-known public Minecraft: Java servers.
//
// Only public connection addresses + plain-text names are shipped here (no
// logos, no "partner"/"featured" claims, no endorsement implied) — see the
// disclaimer on the Servers page. Names are trademarks of their respective
// owners and are used nominatively to identify each server. Edit freely.

export interface FeaturedServer {
  name: string;
  address: string;
  /** One-line, factual description of what the server offers. */
  description: string;
  /** Short gameplay tags. */
  tags: string[];
  /** Optional site for players who want the official source. */
  website?: string;
}

export const FEATURED_SERVERS: FeaturedServer[] = [
  {
    name: "Hypixel",
    address: "mc.hypixel.net",
    description: "The largest minigame network — Bed Wars, SkyBlock, and more.",
    tags: ["Minigames", "SkyBlock"],
    website: "https://hypixel.net",
  },
  {
    name: "Wynncraft",
    address: "play.wynncraft.com",
    description: "A long-running open-world MMORPG built inside Minecraft.",
    tags: ["MMORPG", "Adventure"],
    website: "https://wynncraft.com",
  },
  {
    name: "Complex Gaming",
    address: "hub.mc-complex.com",
    description: "Multi-gamemode network best known for its Pixelmon servers.",
    tags: ["Pixelmon", "Survival"],
    website: "https://www.mc-complex.com",
  },
  {
    name: "ManaCube",
    address: "play.manacube.net",
    description: "Parkour, Islands (SkyBlock), Survival and seasonal events.",
    tags: ["Parkour", "SkyBlock"],
    website: "https://manacube.com",
  },
  {
    name: "Purple Prison",
    address: "purpleprison.net",
    description: "One of the most popular Minecraft prison servers.",
    tags: ["Prison", "Economy"],
    website: "https://purpleprison.com",
  },
];
