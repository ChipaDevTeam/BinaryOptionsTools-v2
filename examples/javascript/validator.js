// Validators decide which raw WebSocket messages a handler keeps.
// This example needs no connection and no ssid.
//
//   node validator.js

const { Validator } = require("../../nodejs");

const none = new Validator();
const regex = Validator.regex("([A-Z])\\w+");
const start = Validator.startsWith("Hello");
const end = Validator.endsWith("Bye");
const contains = Validator.contains("World");
const negated = Validator.ne(contains);

// Needs both a capitalised word and a "Hello" prefix.
const all = Validator.all([regex, start]);
// Needs either "World" anywhere or a "Bye" suffix.
const any = Validator.any([contains, end]);

console.log(`None validator: ${none.check("hello")} (Expected: true)`);

console.log(`Regex validator: ${regex.check("Hello")} (Expected: true)`);
console.log(`Regex validator: ${regex.check("hello")} (Expected: false)`);

console.log(`Starts_with validator: ${start.check("Hello World")} (Expected: true)`);
console.log(`Starts_with validator: ${start.check("hi World")} (Expected: false)`);

console.log(`Ends_with validator: ${end.check("Hello Bye")} (Expected: true)`);
console.log(`Ends_with validator: ${end.check("Hello there")} (Expected: false)`);

console.log(`Contains validator: ${contains.check("Hello World")} (Expected: true)`);
console.log(`Contains validator: ${contains.check("Hello there")} (Expected: false)`);

console.log(`Not validator: ${negated.check("Hello World")} (Expected: false)`);
console.log(`Not validator: ${negated.check("Hello there")} (Expected: true)`);

console.log(`All validator: ${all.check("Hello World")} (Expected: true)`);
console.log(`All validator: ${all.check("hello World")} (Expected: false)`);
console.log(`All validator: ${all.check("Hey there")} (Expected: false)`);

console.log(`Any validator: ${any.check("Hello World")} (Expected: true)`);
console.log(`Any validator: ${any.check("Hello Bye")} (Expected: true)`);
console.log(`Any validator: ${any.check("Hello there")} (Expected: false)`);
