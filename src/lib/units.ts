// Display-only unit conversion between US customary and metric.
// Saved recipe text is never changed: the recipe page shows converted text while a
// mode other than "Auto" is picked, and the original is always one click away.

export type UnitMode = "auto" | "metric" | "us";
export type System = "us" | "metric" | "mixed" | "none";

type Kind = "volume" | "weight";
interface Unit {
  key: string;
  system: "us" | "metric" | "neutral";
  kind: Kind;
  /** Millilitres or grams per one of this unit. */
  base: number;
  names: string[];
}

// tsp/tbsp are used by both systems, so they are left as they are in either direction.
const UNITS: Unit[] = [
  { key: "tsp", system: "neutral", kind: "volume", base: 5, names: ["teaspoons", "teaspoon", "tsps", "tsp"] },
  { key: "tbsp", system: "neutral", kind: "volume", base: 15, names: ["tablespoons", "tablespoon", "tbsps", "tbsp", "tbls", "tbl", "tbs"] },
  { key: "cup", system: "us", kind: "volume", base: 240, names: ["cups", "cup", "c"] },
  { key: "floz", system: "us", kind: "volume", base: 30, names: ["fluid ounces", "fluid ounce", "fl. oz.", "fl. oz", "fl oz", "fl.oz"] },
  { key: "pint", system: "us", kind: "volume", base: 473, names: ["pints", "pint", "pts", "pt"] },
  { key: "quart", system: "us", kind: "volume", base: 946, names: ["quarts", "quart", "qts", "qt"] },
  { key: "gallon", system: "us", kind: "volume", base: 3785, names: ["gallons", "gallon", "gal"] },
  { key: "oz", system: "us", kind: "weight", base: 28.35, names: ["ounces", "ounce", "oz"] },
  { key: "lb", system: "us", kind: "weight", base: 453.6, names: ["pounds", "pound", "lbs", "lb"] },
  { key: "ml", system: "metric", kind: "volume", base: 1, names: ["millilitres", "milliliters", "millilitre", "milliliter", "ml", "mls"] },
  { key: "cl", system: "metric", kind: "volume", base: 10, names: ["centilitres", "centiliters", "cl"] },
  { key: "dl", system: "metric", kind: "volume", base: 100, names: ["decilitres", "deciliters", "dl"] },
  { key: "l", system: "metric", kind: "volume", base: 1000, names: ["litres", "liters", "litre", "liter", "l"] },
  { key: "mg", system: "metric", kind: "weight", base: 0.001, names: ["milligrams", "milligram", "mg"] },
  { key: "g", system: "metric", kind: "weight", base: 1, names: ["grams", "gram", "grs", "gr", "g"] },
  { key: "kg", system: "metric", kind: "weight", base: 1000, names: ["kilograms", "kilogram", "kgs", "kg", "kilos", "kilo"] },
];

const BY_NAME = new Map<string, Unit>();
for (const u of UNITS) for (const n of u.names) BY_NAME.set(n, u);
const NAMES = [...BY_NAME.keys()].sort((a, b) => b.length - a.length).map((n) => n.replace(/[.]/g, "\\."));

const VULGAR: Record<string, number> = { "½": 0.5, "⅓": 1 / 3, "⅔": 2 / 3, "¼": 0.25, "¾": 0.75, "⅛": 0.125, "⅜": 0.375, "⅝": 0.625, "⅞": 0.875, "⅕": 0.2, "⅙": 1 / 6 };
const V = Object.keys(VULGAR).join("");
const NUM = `(?:\\d+\\s+\\d+/\\d+|\\d+\\s*[${V}]|\\d+/\\d+|\\d+(?:[.,]\\d+)?|[${V}])`;
const QTY_RE = new RegExp(
  `(^|[^\\w./])(${NUM})(?:\\s*(?:-|–|—|to)\\s*(${NUM}))?\\s*(${NAMES.join("|")})(?![a-z])`,
  "gi",
);
const TEMP_RE = /(\d{2,3})\s*(?:°|º|degrees?\s*)\s*([CF])(?![a-z])/gi;

export function parseNumber(s: string): number | null {
  const t = s.trim().replace(",", ".");
  let m = t.match(/^(\d+)\s+(\d+)\/(\d+)$/);
  if (m) return +m[1] + +m[2] / +m[3];
  m = t.match(new RegExp(`^(\\d+)\\s*([${V}])$`));
  if (m) return +m[1] + VULGAR[m[2]];
  m = t.match(/^(\d+)\/(\d+)$/);
  if (m) return +m[2] ? +m[1] / +m[2] : null;
  if (VULGAR[t] !== undefined) return VULGAR[t];
  const n = parseFloat(t);
  return Number.isFinite(n) ? n : null;
}

function unitOf(name: string): Unit | undefined {
  return BY_NAME.get(name.toLowerCase());
}

/** Which measuring system a set of lines uses. */
export function detectSystem(lines: string[]): System {
  let us = 0;
  let metric = 0;
  for (const line of lines) {
    for (const m of line.matchAll(QTY_RE)) {
      const u = unitOf(m[4]);
      if (u?.system === "us") us++;
      if (u?.system === "metric") metric++;
    }
    for (const m of line.matchAll(TEMP_RE)) (m[2].toUpperCase() === "F" ? us++ : metric++);
  }
  if (us && metric) return "mixed";
  if (us) return "us";
  if (metric) return "metric";
  return "none";
}

// ---------- formatting ----------

function trimNum(n: number, dp: number) {
  return String(+n.toFixed(dp));
}

function roundMetric(v: number, kind: Kind): string {
  if (kind === "volume" && v >= 1000) return `${trimNum(v / 1000, v >= 10000 ? 1 : 2)} L`;
  if (kind === "weight" && v >= 1000) return `${trimNum(v / 1000, v >= 10000 ? 1 : 2)} kg`;
  // Friendly steps: 225 g rather than 230 g, 900 g rather than 910 g, 960 ml rather than 950 ml.
  const step = v < 10 ? 1 : v < 250 ? 5 : v < 500 ? 10 : kind === "weight" ? 25 : 10;
  const r = Math.max(step === 1 ? 1 : step, Math.round(v / step) * step);
  return `${r} ${kind === "volume" ? "ml" : "g"}`;
}

const FRACTIONS: [number, string][] = [
  [0, ""], [1 / 8, "⅛"], [1 / 4, "¼"], [1 / 3, "⅓"], [1 / 2, "½"], [2 / 3, "⅔"], [3 / 4, "¾"], [1, ""],
];
function fraction(v: number): string {
  let whole = Math.floor(v);
  const rest = v - whole;
  // Eighths only make sense for small amounts; "4⅛ cups" reads oddly.
  const choices = v >= 2 ? FRACTIONS.filter((f) => f[1] !== "⅛") : FRACTIONS;
  let best = choices[0];
  for (const f of choices) if (Math.abs(f[0] - rest) < Math.abs(best[0] - rest)) best = f;
  if (best[0] === 1) whole += 1;
  const frac = best[0] === 1 ? "" : best[1];
  if (!whole && !frac) return "⅛";
  return `${whole || ""}${frac}`;
}

function plural(n: string, one: string, many: string) {
  return n === "1" || /^[⅛¼⅓½⅔¾]$/.test(n) ? one : many;
}

function toUS(v: number, kind: Kind): string {
  if (kind === "weight") {
    if (v < 453.6) {
      const oz = v / 28.35;
      return `${oz < 4 ? fraction(oz) : trimNum(Math.round(oz * 2) / 2, 1)} oz`;
    }
    const lb = Math.floor(v / 453.6);
    const oz = Math.round((v - lb * 453.6) / 28.35);
    if (oz === 16) return `${lb + 1} lb`;
    return oz ? `${lb} lb ${oz} oz` : `${lb} lb`;
  }
  if (v < 15) {
    // Spoon sets come in ⅛, ¼, ½ and ¾, so never ⅓ or ⅔ of a spoon.
    const tsp = v / 5;
    const n = fraction(tsp < 1 ? Math.max(0.125, Math.round(tsp * 8) / 8) : Math.round(tsp * 4) / 4);
    return `${n} tsp`;
  }
  if (v < 60) {
    const n = fraction(Math.max(0.25, Math.round((v / 15) * 4) / 4));
    return `${n} tbsp`;
  }
  if (v <= 1900) {
    const n = fraction(v / 240);
    return `${n} ${plural(n, "cup", "cups")}`;
  }
  const n = trimNum(Math.round((v / 946) * 4) / 4, 2);
  return `${n} ${plural(n, "quart", "quarts")}`;
}

// ---------- dry ingredients ----------

// Grams per US cup (240 ml) for ingredients that metric recipes weigh rather than pour.
// Checked in order, so specific names come before general ones; null means "a liquid, keep ml".
const DENSITIES: [string[], number | null][] = [
  [["buttermilk", "milk", "cream", "water", "stock", "broth", "juice", "oil", "vinegar", "wine", "sauce"], null],
  [["peanut butter", "almond butter", "nut butter"], 270],
  [["butter", "margarine", "shortening"], 227],
  [["almond flour", "almond meal", "ground almonds"], 96],
  [["coconut flour"], 112],
  [["whole wheat flour", "wholemeal flour", "whole-wheat flour", "rye flour", "spelt flour"], 113],
  [["cake flour", "pastry flour"], 115],
  [["cornstarch", "cornflour", "corn starch", "potato starch", "arrowroot"], 112],
  [["cornmeal", "polenta", "semolina"], 150],
  [["flour"], 120],
  [["brown sugar", "muscovado"], 213],
  [["powdered sugar", "icing sugar", "confectioners", "confectioner's"], 120],
  [["coconut sugar"], 160],
  [["sugar", "caster"], 200],
  [["honey", "molasses", "treacle", "golden syrup", "corn syrup"], 340],
  [["maple syrup"], 312],
  [["cocoa"], 85],
  [["rolled oats", "oats", "oatmeal"], 90],
  [["rice"], 190],
  [["chocolate chips", "chocolate chunks", "chopped chocolate"], 170],
  [["shredded coconut", "desiccated coconut", "coconut flakes"], 85],
  [["parmesan", "pecorino"], 100],
  [["shredded cheese", "grated cheese", "cheddar", "mozzarella"], 113],
  [["panko"], 50],
  [["breadcrumbs", "bread crumbs"], 112],
  [["raisins", "sultanas", "dried cranberries"], 150],
  [["walnuts", "pecans", "almonds", "hazelnuts", "nuts"], 120],
  [["yogurt", "yoghurt", "sour cream", "cream cheese", "ricotta"], 240],
];

/** Grams per cup for the ingredient named right after an amount, or null to keep volumes. */
export function densityFor(after: string): number | null {
  // Only look at the ingredient this amount belongs to, not the rest of the sentence.
  const words = after.toLowerCase().split(/[,;.()]|\band\b|\bor\b|\binto\b|\bwith\b/)[0].slice(0, 60);
  for (const [names, grams] of DENSITIES) {
    if (names.some((n) => new RegExp(`(^|[^a-z])${n.replace(/[.*+?^${}()|[\]\\']/g, "\\$&")}`).test(words))) return grams;
  }
  return null;
}

function convertAmount(value: number, unit: Unit, to: "us" | "metric", gramsPerCup: number | null): string | null {
  if (unit.system === "neutral" || unit.system === to) return null;
  const base = value * unit.base;
  if (gramsPerCup !== null) {
    // Dry ingredient: cups <-> grams.
    if (to === "metric" && unit.kind === "volume") return roundMetric((base / 240) * gramsPerCup, "weight");
    if (to === "us" && unit.kind === "weight") return toUS((base / gramsPerCup) * 240, "volume");
  }
  return to === "metric" ? roundMetric(base, unit.kind) : toUS(base, unit.kind);
}

function convertRange(a: number, b: number | null, unit: Unit, to: "us" | "metric", gramsPerCup: number | null): string | null {
  const first = convertAmount(a, unit, to, gramsPerCup);
  if (!first) return null;
  if (b === null) return first;
  const second = convertAmount(b, unit, to, gramsPerCup);
  if (!second) return null;
  // "1–2 cups" -> "240–480 ml": keep one unit when both ends share it.
  const [n1, ...u1] = first.split(" ");
  const [n2, ...u2] = second.split(" ");
  return u1.join(" ") === u2.join(" ") ? `${n1}–${n2} ${u2.join(" ")}` : `${first} – ${second}`;
}

/**
 * Converts every amount in a line of text. Returns the text unchanged for "auto",
 * and leaves lines that already give both systems (e.g. "1 cup (240 ml)") alone.
 */
export function convertText(text: string, mode: UnitMode): string {
  if (mode === "auto" || !text) return text;
  const to = mode;
  if (detectSystem([text]) === "mixed") return text;

  let out = text.replace(QTY_RE, (whole, lead: string, a: string, b: string | undefined, name: string, offset: number, all: string) => {
    const unit = unitOf(name);
    const va = parseNumber(a);
    const vb = b ? parseNumber(b) : null;
    if (!unit || va === null || (b && vb === null)) return whole;
    const after = all.slice(offset + whole.length);
    const conv = convertRange(va, vb, unit, to, densityFor(after));
    return conv ? `${lead}${conv}` : whole;
  });
  out = out.replace(TEMP_RE, (whole, deg: string, scale: string) => {
    const s = scale.toUpperCase();
    const d = +deg;
    if (to === "metric" && s === "F") return `${Math.round(((d - 32) * 5) / 9 / 5) * 5}°C`;
    if (to === "us" && s === "C") return `${Math.round(((d * 9) / 5 + 32) / 5) * 5}°F`;
    return whole;
  });
  return out;
}

export function systemLabel(s: System): string {
  return { us: "US measurements", metric: "metric measurements", mixed: "both US and metric", none: "no measurements to convert" }[s];
}

const MODE_KEY = "rc.unitMode";
export function loadUnitMode(): UnitMode {
  try {
    const m = localStorage.getItem(MODE_KEY);
    if (m === "metric" || m === "us" || m === "auto") return m;
  } catch {}
  return "auto";
}
export function saveUnitMode(m: UnitMode) {
  try {
    localStorage.setItem(MODE_KEY, m);
  } catch {}
}
