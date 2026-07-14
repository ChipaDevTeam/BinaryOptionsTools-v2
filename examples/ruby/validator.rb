# Validator class usage example
require_relative 'binary_options_tools_uni'

# Create validator instances
none = BinaryOptionsToolsUni::Validator.new
regex = BinaryOptionsToolsUni::Validator.regex("([A-Z])\\w+")
start = BinaryOptionsToolsUni::Validator.starts_with("Hello")
ends = BinaryOptionsToolsUni::Validator.ends_with("Bye")
contains = BinaryOptionsToolsUni::Validator.contains("World")
rnot = BinaryOptionsToolsUni::Validator.ne(contains)

# Combined validators
rall = BinaryOptionsToolsUni::Validator.all([regex, start])
rany = BinaryOptionsToolsUni::Validator.any([contains, ends])

# Testing each validator
puts "None validator: #{none.check("hello")} (Expected: true)"
puts "Regex validator: #{regex.check("Hello")} (Expected: true)"
puts "Regex validator: #{regex.check("hello")} (Expected: false)"
puts "Starts_with validator: #{start.check("Hello World")} (Expected: true)"
puts "Starts_with validator: #{start.check("hi World")} (Expected: false)"
puts "Ends_with validator: #{ends.check("Hello Bye")} (Expected: true)"
puts "Ends_with validator: #{ends.check("Hello there")} (Expected: false)"
puts "Contains validator: #{contains.check("Hello World")} (Expected: true)"
puts "Contains validator: #{contains.check("Hello there")} (Expected: false)"
puts "Not validator: #{rnot.check("Hello World")} (Expected: false)"
puts "Not validator: #{rnot.check("Hello there")} (Expected: true)"

# Testing the all validator
puts "All validator: #{rall.check("Hello World")} (Expected: true)"
puts "All validator: #{rall.check("hello World")} (Expected: false)"
puts "All validator: #{rall.check("Hey there")} (Expected: false)"

# Testing the any validator
puts "Any validator: #{rany.check("Hello World")} (Expected: true)"
puts "Any validator: #{rany.check("Hello Bye")} (Expected: true)"
puts "Any validator: #{rany.check("Hello there")} (Expected: false)"
