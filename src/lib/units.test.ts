// Run with: node --experimental-strip-types src/lib/units.test.ts
import { convertText, detectSystem, parseNumber } from "./units.ts";

let failed = 0;
function eq(actual: unknown, expected: unknown, label: string) {
  if (actual !== expected) {
    failed++;
    console.log(`FAIL ${label}\n  expected: ${expected}\n  actual:   ${actual}`);
  }
}

// Numbers
eq(parseNumber("1 1/2"), 1.5, "mixed fraction");
eq(parseNumber("1½"), 1.5, "vulgar with whole");
eq(parseNumber("¾"), 0.75, "vulgar");
eq(parseNumber("1,5"), 1.5, "comma decimal");
eq(parseNumber("2/3"), 2 / 3, "fraction");

// Detection
eq(detectSystem(["2 cups flour", "1 tsp salt"]), "us", "detect us");
eq(detectSystem(["250 g flour", "500 ml milk"]), "metric", "detect metric");
eq(detectSystem(["1 cup (240 ml) milk"]), "mixed", "detect mixed");
eq(detectSystem(["2 eggs", "1 tsp vanilla"]), "none", "tsp is neutral");
eq(detectSystem(["Bake at 350°F"]), "us", "detect from temperature");

// US -> metric
eq(convertText("2 cups flour", "metric"), "480 ml flour", "cups");
eq(convertText("1/2 cup sugar", "metric"), "120 ml sugar", "half cup");
eq(convertText("1 1/2 cups milk", "metric"), "360 ml milk", "mixed number");
eq(convertText("8 oz cream cheese", "metric"), "225 g cream cheese", "ounces");
eq(convertText("2 lb chicken thighs", "metric"), "900 g chicken thighs", "pounds");
eq(convertText("3 lbs potatoes", "metric"), "1.36 kg potatoes", "kilograms");
eq(convertText("1-2 cups stock", "metric"), "240–480 ml stock", "range");
eq(convertText("1 tbsp olive oil", "metric"), "1 tbsp olive oil", "tbsp untouched");
eq(convertText("2 large eggs", "metric"), "2 large eggs", "no unit");
eq(convertText("Bake at 350°F for 25 minutes.", "metric"), "Bake at 175°C for 25 minutes.", "oven temp");
eq(convertText("Preheat oven to 400 degrees F", "metric"), "Preheat oven to 205°C", "degrees F");
eq(convertText("Whisk 1 cup of the milk into 2 cups flour", "metric"), "Whisk 240 ml of the milk into 480 ml flour", "inside sentence");
eq(convertText("1 cup (240 ml) milk", "metric"), "1 cup (240 ml) milk", "both given, untouched");
eq(convertText("4 cups chopped kale", "metric"), "960 ml chopped kale", "4 cups");
eq(convertText("6 cups water", "metric"), "1.44 L water", "litres");

// metric -> US
eq(convertText("250 g butter", "us"), "9 oz butter", "grams");
eq(convertText("500 g flour", "us"), "1 lb 2 oz flour", "lb and oz");
eq(convertText("1 kg potatoes", "us"), "2 lb 3 oz potatoes", "kilo");
eq(convertText("250 ml milk", "us"), "1 cup milk", "ml to cup");
eq(convertText("125ml cream", "us"), "½ cup cream", "no space");
eq(convertText("30 ml lemon juice", "us"), "2 tbsp lemon juice", "small ml to tbsp");
eq(convertText("1 l stock", "us"), "4¼ cups stock", "litre");
eq(convertText("2 litres water", "us"), "2 quarts water", "litres to quarts");
eq(convertText("1,5 kg pork", "us"), "3 lb 5 oz pork", "comma decimal kg");
eq(convertText("Bake at 180°C", "us"), "Bake at 355°F", "celsius");
eq(convertText("3 garlic cloves", "us"), "3 garlic cloves", "g inside a word");
eq(convertText("2 large onions", "us"), "2 large onions", "l inside a word");

// Auto leaves everything alone
eq(convertText("2 cups flour", "auto"), "2 cups flour", "auto");

if (failed) {
  console.log(`\n${failed} failed`);
  process.exit(1);
}
console.log("All unit tests passed");
