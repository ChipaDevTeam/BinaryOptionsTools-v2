const { Validator } = require("./binary-options-tools.node");

// Create validator instances
const none = new Validator();
const regex = Validator.regex(/([A-Z])\w+/);
const start = Validator.startsWith("Hello");
const end = Validator.endsWith("Bye");
const contains = Validator.contains("World");
const rnot = Validator.not(contains);

// Modified for better testing - smaller groups with predictable outcomes
const rall = Validator.all([regex, start]); // Will need both capital letter and "Hello" at start
const rany = Validator.any([contains, end]); // Will need either "World" or end with "Bye"

// Testing each validator
console.log(`None validator: ${none.check("hello")} (Expected: true)`);

console.log(`Regex validator: ${regex.check("Hello")} (Expected: true)`);
console.log(`Regex validator: ${regex.check("hello")} (Expected: false)`);

console.log(
  `Starts_with validator: ${start.check("Hello World")} (Expected: true)`,
);
console.log(
  `Starts_with validator: ${start.check("hi World")} (Expected: false)`,
);

console.log(`Ends_with validator: ${end.check("Hello Bye")} (Expected: true)`);
console.log(
  `Ends_with validator: ${end.check("Hello there")} (Expected: false)`,
);

console.log(
  `Contains validator: ${contains.check("Hello World")} (Expected: true)`,
);
console.log(
  `Contains validator: ${contains.check("Hello there")} (Expected: false)`,
);

console.log(`Not validator: ${rnot.check("Hello World")} (Expected: false)`);
console.log(`Not validator: ${rnot.check("Hello there")} (Expected: true)`);

// Testing the all validator
console.log(`All validator: ${rall.check("Hello World")} (Expected: true)`); // Starts with "Hello" and has capital
console.log(`All validator: ${rall.check("hello World")} (Expected: false)`); // No capital at start
console.log(`All validator: ${rall.check("Hey there")} (Expected: false)`); // Has capital but doesn't start with "Hello"

// Testing the any validator
console.log(`Any validator: ${rany.check("Hello World")} (Expected: true)`); // Contains "World"
console.log(`Any validator: ${rany.check("Hello Bye")} (Expected: true)`); // Ends with "Bye"
console.log(`Any validator: ${rany.check("Hello there")} (Expected: false)`); // Neither contains "World" nor ends with "Bye"
