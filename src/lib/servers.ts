// Default quick-connect servers. Public addresses + plain names only. Edit freely.

export interface FeaturedServer {
  name: string;
  address: string;
  description: string;
  tags: string[];
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
