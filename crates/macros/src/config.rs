use proc_macro2::TokenStream as TokenStream2;

use darling::{ast, util, FromDeriveInput, FromField, FromMeta};
use quote::{quote, ToTokens};
use syn::{Generics, Ident, Type};

// Step 1: Parsing attributes into intermediate structs.
// `FieldConfig` defines special configurations that can be applied to a field
// using the `#[config(...)]` attribute.
#[derive(Debug, FromMeta)]
enum FieldConfig {
    // `#[config(optional)]`: Marks a field as optional in the builder.
    // When building the config struct, if this field is None in the builder,
    // it will default to `None` in the `Arc<Mutex<Option<T>>>`.
    #[darling(rename = "optional")]
    Optional,
    // `#[config(iterator(dtype = Type, add_fn = "function_name"))]`:
    // Marks a field as a collection and generates an `add_` method.
    // `dtype`: Specifies the type of elements in the collection.
    // `add_fn`: Optionally specifies the method name to add elements (e.g., "push", "insert"). Defaults to "push".
    #[darling(rename = "iterator")]
    Iterator {
        dtype: Box<Type>,
        add_fn: Option<String>,
    },
}

// `ConfigField` represents a single field from the input struct.
// It's derived using `darling::FromField` to parse field-level attributes.
#[derive(Debug, FromField)]
#[darling(attributes(config))] // Specifies that attributes for this field are under `#[config(...)]`
struct ConfigField {
    ident: Option<Ident>, // The identifier (name) of the field.
    ty: Type,             // The type of the field.
    // `extra`: Captures any `FieldConfig` applied to this field via `#[config(...)]`.
    extra: Option<FieldConfig>,
}

// `Config` represents the entire struct to which the `#[derive(Config)]` macro is applied.
// It's derived using `darling::FromDeriveInput`.
#[derive(Debug, FromDeriveInput)]
#[darling(attributes(config), supports(struct_named))] // Specifies struct-level attributes and that it only works on named structs.
pub struct Config {
    ident: Ident, // The identifier (name) of the struct.
    // `data`: Contains the fields of the struct, parsed into `ConfigField`.
    // `ast::Data<util::Ignored, ConfigField>` means we only care about struct fields,
    // and those fields are parsed into `ConfigField`.
    data: ast::Data<util::Ignored, ConfigField>,
    generics: Generics, // Generics of the input struct (e.g., `<T>`).
}

// Step 2: Generating Rust code (TokenStream2) from the parsed intermediate structs.
// `impl ToTokens for Config` is the main entry point for code generation for the entire struct.
impl ToTokens for Config {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        // Extract the fields from the parsed struct data.
        // `take_struct()` ensures we are dealing with a struct with named fields.
        let fields = &self
            .data
            .as_ref()
            .take_struct()
            .expect("Only available for structs");

        // `name`: The original struct's identifier.
        let name = &self.ident;

        // `new_name`: The identifier for the generated config struct.
        // If the original struct name starts with `_`, it's removed. Otherwise, "Config" is appended.
        // e.g., `MyStruct` -> `MyStructConfig`, `_Internal` -> `InternalConfig`.
        let new_name = match format!("{name}") {
            n if n.starts_with("_") => Ident::new(&n[1..], name.span()),
            n => Ident::new(&format!("{n}Config"), name.span()),
        };

        // `builder_name`: The identifier for the generated builder struct.
        // e.g., `MyStructConfig` -> `MyStructConfigBuilder`.
        let builder_name = Ident::new(&format!("{new_name}Builder"), new_name.span());

        // --- Preparing iterators for code generation ---
        // `fields_builders`: Generates the builder methods for each field (e.g., `fn field_name(self, value: Type) -> Self`).
        let fields_builders = fields.iter().map(|f| f.builder());
        // `fn_iter`: Generates getter/setter/adder methods for each field in the config struct.
        // This delegates to `ConfigField::to_tokens`.
        let fn_iter = fields.iter();
        // `field_names*`: Iterators over the field identifiers, used in various parts of the generated code
        // for defining struct fields, initializing them, etc.
        let field_names = fields.iter().filter_map(|f| f.ident.as_ref());
        let field_names2 = field_names.clone();
        let field_names3 = field_names.clone();
        let field_names4 = field_names.clone();
        let field_names5 = field_names.clone();
        // `ok_or_error`: Generates the logic for initializing fields in the config struct from the builder.
        // Handles required fields (panic if None), optional fields, and iterator fields (default if None).
        let ok_or_error = fields.iter().map(|f| f.ok_panic_default());
        // `field_none`: Generates `field_name: None::<Type>` for initializing builder fields to `None`.
        let field_none = fields.iter().map(|f| f.field_none());
        // `field_type*`: Iterators over field types.
        let field_type = fields.iter().map(|f| &f.ty);
        let field_type2 = field_type.clone();

        // `generics`: Original struct's generics.
        let generics = &self.generics;
        // `split_for_impl`: Splits generics into parts needed for `impl` blocks (e.g., `impl<T>`, ` <T>`, `where T: Clone`).
        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

        // --- Code Generation using `quote!` ---
        // The `quote!` macro takes Rust-like syntax and generates `TokenStream2`.
        // Variables prefixed with `#` are interpolated. `#(...)*` repeats for each item in an iterator.
        tokens.extend(quote! {
            // Define the generated Config struct (e.g., `MyStructConfig`).
            // Each field is wrapped in `Arc<Mutex<T>>` to allow shared mutable access
            // across different parts of an application, ensuring thread safety.
            #[derive(Clone)] // Clone is derived to allow cloning the config (which clones the Arcs).
            pub struct #new_name #generics {
                #(#field_names: ::std::sync::Arc<::std::sync::Mutex<#field_type>>),*
            }

            // Define the generated Builder struct (e.g., `MyStructConfigBuilder`).
            // Each field is an `Option<T>`, allowing for partial construction.
            // Fields are set individually, and then `build()` is called.
            pub struct #builder_name #generics {
                #(#field_names2: ::std::option::Option<#field_type2>),*
            }

            // Implement a `builder()` method on the original struct.
            // This allows transitioning from an instance of the original struct to its builder.
            // e.g., `let my_struct_builder = my_struct_instance.builder();`
            impl #impl_generics #name #ty_generics #where_clause {
                pub fn builder(self) -> #builder_name #ty_generics {
                    #builder_name::from(self) // Delegates to `From<OriginalStruct> for BuilderStruct`
                }
            }

            // Implement methods (getters, setters, adders) on the generated Config struct.
            // This iterates through `fn_iter` which calls `ConfigField::to_tokens` for each field.
            impl #impl_generics #new_name #ty_generics #where_clause {
                #(#fn_iter)*
            }

            // Implement methods on the generated Builder struct.
            impl #impl_generics #builder_name #ty_generics #where_clause {
                // Field setter methods for the builder (fluent interface).
                // e.g., `builder.field1(value1).field2(value2)`
                #(#fields_builders)*

                // `new()`: Constructor for the builder, initializing all fields to `None`.
                pub fn new() -> #builder_name #ty_generics {
                    Self {
                        #(#field_none),* // Initializes each field_name: Option::None::<FieldType>
                    }
                }

                // `build()`: Consumes the builder and attempts to create an instance of the Config struct.
                // Returns `anyhow::Result` to handle potential errors (e.g., a required field not set).
                pub fn build(self) -> ::anyhow::Result<#new_name #ty_generics> {
                    #new_name::try_from(self) // Delegates to `TryFrom<BuilderStruct> for ConfigStruct`
                }
            }

            // Implement `Default` for the Builder struct, making `Builder::new()` the default.
            impl #impl_generics ::std::default::Default for #builder_name #ty_generics #where_clause {
                fn default() -> Self {
                    Self::new()
                }
            }

            // Implement `From<OriginalStruct> for ConfigStruct`.
            // Converts an instance of the original struct directly into a Config struct.
            // Each field from the original struct is wrapped in `Arc::new(Mutex::new(...))`.
            impl #impl_generics From<#name #ty_generics> for #new_name #ty_generics #where_clause {
                fn from(value: #name #ty_generics) -> Self {
                    Self {
                        #(#field_names3: ::std::sync::Arc::new(::std::sync::Mutex::new(value.#field_names3))),*
                    }
                }
            }

            // Implement `From<OriginalStruct> for BuilderStruct`.
            // Converts an instance of the original struct into a Builder struct.
            // Each field from the original struct is wrapped in `Some(...)`.
            impl #impl_generics From<#name #ty_generics> for #builder_name #ty_generics #where_clause {
                fn from(value: #name #ty_generics) -> Self {
                    Self {
                        #(#field_names4: ::std::option::Option::Some(value.#field_names4)),*
                    }
                }
            }

            // Implement `TryFrom<ConfigStruct> for OriginalStruct`.
            // Converts a Config struct back into an instance of the original struct.
            // This involves locking each Mutex and cloning the inner value.
            // Returns `Result` because locking a Mutex can fail (if poisoned).
            impl #impl_generics TryFrom<#new_name #ty_generics> for #name #ty_generics #where_clause {
                type Error = ::anyhow::Error;

                fn try_from(value: #new_name #ty_generics) -> ::std::result::Result<Self, Self::Error> {
                    Ok(
                        Self {
                            // For each field, lock the mutex, handle potential poison error, and clone the value.
                            #(#field_names5: value.#field_names5.lock().map_err(|e| ::anyhow::anyhow!("Poison error {e}"))?.clone()),*
                        }
                    )
                }
            }

            // Implement `TryFrom<BuilderStruct> for ConfigStruct`.
            // This is the core logic for building the Config struct from the Builder.
            // It uses `ok_or_error` (which calls `ConfigField::ok_panic_default`) to handle
            // how each field is initialized based on its configuration (required, optional, iterator).
            impl #impl_generics TryFrom<#builder_name #ty_generics> for #new_name #ty_generics #where_clause {
                type Error = ::anyhow::Error;

                fn try_from(value: #builder_name #ty_generics) -> ::std::result::Result<Self, Self::Error> {
                    Ok(
                        Self {
                            // `ok_or_error` generates the initialization logic for each field.
                            // e.g., for a required field: `Arc::new(Mutex::new(value.field_name.ok_or("error")?))`
                            // e.g., for an optional field: `Arc::new(Mutex::new(value.field_name.unwrap_or(None)))`
                            // e.g., for an iterator field: `Arc::new(Mutex::new(value.field_name.unwrap_or_default())))`
                            #(#ok_or_error),*
                        }
                    )
                }
            }
        });
    }
}

// `impl ToTokens for ConfigField` generates the methods for a single field
// within the `impl ConfigStruct { ... }` block.
impl ToTokens for ConfigField {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let name = self.ident.as_ref().expect("Only fields with ident allowed");
        let dtype = &self.ty; // The type of the field, e.g., `String`, `Vec<u32>`, `Option<i32>`.

        // Generate `set_field_name` method.
        let set_name = Ident::new(&format!("set_{name}"), name.span());
        // Generate `get_field_name` method.
        let get_name = Ident::new(&format!("get_{name}"), name.span());

        // `extra`: Handles special code generation for `Iterator` fields.
        let extra = if let Some(FieldConfig::Iterator {
            dtype: iterator_item_type,
            add_fn,
        }) = &self.extra
        {
            // If the field is configured as an iterator `#[config(iterator(dtype = ...))]`

            // `add_name`: Name of the method to add items, e.g., `add_my_vec`.
            let add_name = Ident::new(&format!("add_{name}"), name.span());
            // `add_fn_ident`: The actual function to call on the collection, e.g., `push`, `insert`.
            // Defaults to `push` if not specified in `#[config(iterator(add_fn = "..."))]`.
            let add_fn_ident = if let Some(add) = add_fn {
                Ident::new(add, name.span())
            } else {
                Ident::new("push", name.span())
            };
            // Generate the `add_field_name` method.
            // It locks the Mutex, calls the specified `add_fn_ident` on the collection, and returns `Result`.
            quote! {
                pub fn #add_name(&self, value: #iterator_item_type) -> ::anyhow::Result<()> {
                    let mut field = self.#name.lock().map_err(|e| ::anyhow::anyhow!("Poison error {e}"))?;
                    field.#add_fn_ident(value); // e.g., field.push(value)
                    Ok(())
                }
            }
        } else {
            // If not an iterator field, no extra methods are generated here.
            quote! {}
        };

        tokens.extend(quote! {
            // Append the `add_` method if generated.
            #extra

            // Generate the `set_field_name` method.
            // It locks the Mutex and replaces the entire value.
            // `value` here is of `dtype` (the full type of the field, e.g., `Vec<String>`).
            pub fn #set_name(&self, value: #dtype) -> ::anyhow::Result<()> {
                let mut field = self.#name.lock().map_err(|e| ::anyhow::anyhow!("Poison error {e}"))?;
                *field = value;
                Ok(())
            }

            // Generate the `get_field_name` method.
            // It locks the Mutex and clones the inner value.
            // Returns `Result<FieldType>` to handle potential Mutex poison errors.
            pub fn #get_name(&self) -> ::anyhow::Result<#dtype> {
                Ok(self.#name.lock().map_err(|e| ::anyhow::anyhow!("Poison error {e}"))?.clone())
            }
        });
    }
}

// Helper methods for `ConfigField` used during token generation by `Config::to_tokens`.
impl ConfigField {
    // `builder()`: Generates the fluent setter method for this field in the Builder struct.
    // e.g., `pub fn field_name(mut self, value: FieldType) -> Self { self.field_name = Some(value); self }`
    fn builder(&self) -> TokenStream2 {
        let name = self.ident.as_ref().expect("should have a name");
        let dtype = &self.ty;
        quote! {
            pub fn #name(mut self, value: #dtype) -> Self {
                self.#name = Some(value);
                self
            }
        }
    }

    // `field_none()`: Generates the initialization for this field in the Builder's `new()` method.
    // e.g., `field_name: ::std::option::Option::None::<FieldType>`
    fn field_none(&self) -> TokenStream2 {
        let name = self.ident.as_ref().expect("should have a name");
        let dtype = &self.ty; // Note: This `dtype` is the full type of the field.
        quote! {
            #name: ::std::option::Option::None::<#dtype>
        }
    }

    // `ok_panic_default()`: Generates the logic for initializing this field in the Config struct
    // when converting `TryFrom<BuilderStruct>`. This is a crucial part that handles
    // different field configurations (`extra: Option<FieldConfig>`).
    fn ok_panic_default(&self) -> TokenStream2 {
        let name = self.ident.as_ref().expect("should have a name");
        let name_str = format!("{name}"); // Field name as a string for error messages.

        if let Some(extra_config) = &self.extra {
            match extra_config {
                // If `#[config(iterator(...))]`:
                // The field in the builder is `Option<CollectionType>`.
                // If `Some(collection)`, use it. If `None`, use `Default::default()` for the collection type.
                // This assumes the collection type implements `Default` (e.g., `Vec::new()`).
                FieldConfig::Iterator { .. } => {
                    quote! {
                        #name: ::std::sync::Arc::new(::std::sync::Mutex::new(value.#name.unwrap_or_else(::std::default::Default::default)))
                    }
                }
                // If `#[config(optional)]`:
                // The field type itself is `Option<InnerType>`. The builder field is `Option<Option<InnerType>>`.
                // `value.#name` is `Option<Option<InnerType>>`.
                // `unwrap_or(Option::None)` means if the builder had `Some(Some(val))` -> `Some(val)`,
                // if `Some(None)` -> `None`, if `None` (builder field not set) -> `None`.
                // The resulting `Arc<Mutex<Option<InnerType>>>` will hold `None` if the builder didn't provide a value.
                FieldConfig::Optional => {
                    // The field's type `self.ty` is expected to be `Option<T>`.
                    // `value.#name` from the builder is `Option<Option<T>>`.
                    // We want `Arc<Mutex<Option<T>>>`.
                    // `value.#name.unwrap_or(::std::option::Option::None)` handles the outer Option from the builder.
                    // If builder's `value.#name` is `None` (field not set), it becomes `Arc<Mutex<None>>`.
                    // If builder's `value.#name` is `Some(actual_option_value)`, it becomes `Arc<Mutex<actual_option_value>>`.
                    quote! {
                        #name: ::std::sync::Arc::new(::std::sync::Mutex::new(value.#name.unwrap_or(::std::option::Option::None)))
                    }
                }
            }
        } else {
            // If no special `#[config(...)]` attribute (i.e., it's a required field):
            // The field in the builder is `Option<FieldType>`.
            // `value.#name.ok_or(...)` ensures that if the builder has `None` for this field,
            // an error is returned, effectively making the field mandatory.
            quote! {
                #name: ::std::sync::Arc::new(::std::sync::Mutex::new(value.#name.ok_or(::anyhow::anyhow!("Option for field '{}' was None", #name_str))?))
            }
        }
    }
}
