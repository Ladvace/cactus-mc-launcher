const ADJECTIVES = [
  "Chunky", "Sneaky", "Blocky", "Explosive", "Mossy", "Enchanted", "Cursed",
  "Glowing", "Frosty", "Pixelated", "Legendary", "Grumpy", "Turbo", "Soggy",
  "Spicy", "Wobbly", "Feral", "Dizzy", "Sleepy", "Rowdy", "Golden", "Molten",
  "Haunted", "Squeaky", "Crunchy", "Diamond", "Redstone", "Obsidian", "Nether",
];

const NOUNS = [
  "Creeper", "Enderman", "Llama", "Piglin", "Villager", "Bastion", "Beacon",
  "Cavern", "Fortress", "Sheep", "Goat", "Axolotl", "Warden", "Allay", "Slime",
  "Zombie", "Wolf", "Parrot", "Bee", "Ghast", "Blaze", "Golem", "Phantom",
  "Turtle", "Panda", "Fox", "Dolphin", "Shulker", "Ravager",
];

const SPECIALS = [
  "Diamonds Are Forever", "Creeper Aw Man", "One More Night Mining",
  "Just One More Diamond", "The Nether Regrets", "Dirt Hut Deluxe",
  "Respawn & Repeat", "404 Diamonds Not Found", "Cobblestone Cathedral",
  "Certified Creeper Farm", "Elytra or Bust", "Bed Wars Survivor",
  "Peaceful Mode Liar", "It's Just A Phase (Nether)",
];

function pick<T>(arr: T[]): T {
  return arr[Math.floor(Math.random() * arr.length)];
}

export function randomInstanceName(): string {
  if (Math.random() < 0.18) return pick(SPECIALS);
  return `${pick(ADJECTIVES)} ${pick(NOUNS)}`;
}
