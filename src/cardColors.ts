// Muted, medium-lightness colors chosen to stay legible as a card accent
// across all four themes (dark, light, hacker, claude) without being harsh.
const PALETTE = [
  "#6b8fb5", // dusty blue
  "#7fa87f", // sage green
  "#b58fc9", // muted lavender
  "#c98f6b", // terracotta
  "#6bb5a8", // soft teal
  "#c9a86b", // ochre
  "#b56b8f", // dusty rose
  "#8f9bb5", // slate blue-gray
  "#a8b56b", // olive
  "#b58f6b", // warm taupe
];

function hashString(s: string): number {
  let h = 0;
  for (let i = 0; i < s.length; i++) {
    h = (h * 31 + s.charCodeAt(i)) >>> 0;
  }
  return h;
}

/** Deterministic per-server accent color — same id always gets the same color. */
export function colorForServer(id: string): string {
  return PALETTE[hashString(id) % PALETTE.length];
}
