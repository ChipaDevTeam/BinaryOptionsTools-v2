// Example demonstrating validation patterns (similar to the Validator class)
package main

import (
	"fmt"
	"regexp"
	"strings"
)

// ValidatorFunc represents a validation function
type ValidatorFunc func(input string) bool

// Validator provides a set of validation constructors matching the Python Validator API
type Validator struct {
	check ValidatorFunc
}

func (v *Validator) Check(input string) bool {
	return v.check(input)
}

// NewValidator creates a default validator that accepts all messages
func NewValidator() *Validator {
	return &Validator{check: func(_ string) bool { return true }}
}

// NewValidatorRegex creates a validator that checks if input matches a regex pattern
func NewValidatorRegex(pattern string) (*Validator, error) {
	re, err := regexp.Compile(pattern)
	if err != nil {
		return nil, err
	}
	return &Validator{check: re.MatchString}, nil
}

// NewValidatorStartsWith creates a validator that checks if input starts with a prefix
func NewValidatorStartsWith(prefix string) *Validator {
	return &Validator{check: func(input string) bool { return strings.HasPrefix(input, prefix) }}
}

// NewValidatorEndsWith creates a validator that checks if input ends with a suffix
func NewValidatorEndsWith(suffix string) *Validator {
	return &Validator{check: func(input string) bool { return strings.HasSuffix(input, suffix) }}
}

// NewValidatorContains creates a validator that checks if input contains a substring
func NewValidatorContains(substring string) *Validator {
	return &Validator{check: func(input string) bool { return strings.Contains(input, substring) }}
}

// NewValidatorNot creates a validator that negates another validator
func NewValidatorNot(v *Validator) *Validator {
	return &Validator{check: func(input string) bool { return !v.check(input) }}
}

// NewValidatorAll creates a validator that passes only if all validators pass
func NewValidatorAll(validators ...*Validator) *Validator {
	return &Validator{check: func(input string) bool {
		for _, v := range validators {
			if !v.check(input) {
				return false
			}
		}
		return true
	}}
}

// NewValidatorAny creates a validator that passes if any validator passes
func NewValidatorAny(validators ...*Validator) *Validator {
	return &Validator{check: func(input string) bool {
		for _, v := range validators {
			if v.check(input) {
				return true
			}
		}
		return false
	}}
}

func main() {
	none := NewValidator()
	regex, _ := NewValidatorRegex("([A-Z])\\w+")
	start := NewValidatorStartsWith("Hello")
	end := NewValidatorEndsWith("Bye")
	contains := NewValidatorContains("World")
	rnot := NewValidatorNot(contains)
	custom := &Validator{check: func(input string) bool {
		return strings.HasPrefix(input, "Hello") && strings.HasSuffix(input, "World")
	}}

	// Combined validators
	rall := NewValidatorAll(regex, start)   // Needs both capital letter and "Hello" at start
	rany := NewValidatorAny(contains, end)  // Needs either "World" or ends with "Bye"

	fmt.Printf("None validator: %t (Expected: true)\n", none.Check("hello"))
	fmt.Printf("Regex validator: %t (Expected: true)\n", regex.Check("Hello"))
	fmt.Printf("Regex validator: %t (Expected: false)\n", regex.Check("hello"))
	fmt.Printf("Starts_with validator: %t (Expected: true)\n", start.Check("Hello World"))
	fmt.Printf("Starts_with validator: %t (Expected: false)\n", start.Check("hi World"))
	fmt.Printf("Ends_with validator: %t (Expected: true)\n", end.Check("Hello Bye"))
	fmt.Printf("Ends_with validator: %t (Expected: false)\n", end.Check("Hello there"))
	fmt.Printf("Contains validator: %t (Expected: true)\n", contains.Check("Hello World"))
	fmt.Printf("Contains validator: %t (Expected: false)\n", contains.Check("Hello there"))
	fmt.Printf("Not validator: %t (Expected: false)\n", rnot.Check("Hello World"))
	fmt.Printf("Not validator: %t (Expected: true)\n", rnot.Check("Hello there"))
	fmt.Printf("Custom validator: %t (Expected: true)\n", custom.Check("Hello World"))
	fmt.Printf("Custom validator: %t (Expected: false)\n", custom.Check("Hello there"))

	// Testing the all validator
	fmt.Printf("All validator: %t (Expected: true)\n", rall.Check("Hello World"))
	fmt.Printf("All validator: %t (Expected: false)\n", rall.Check("hello World"))
	fmt.Printf("All validator: %t (Expected: false)\n", rall.Check("Hey there"))

	// Testing the any validator
	fmt.Printf("Any validator: %t (Expected: true)\n", rany.Check("Hello World"))
	fmt.Printf("Any validator: %t (Expected: true)\n", rany.Check("Hello Bye"))
	fmt.Printf("Any validator: %t (Expected: false)\n", rany.Check("Hello there"))
}
