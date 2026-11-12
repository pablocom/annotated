//! Validation for structs: write rules on their fields, and get every problem back at once.
//!
//! [`Validate::validate`] checks a value. It returns `Ok(())`, or every [`Issue`] it found. Each issue is a
//! [`Problem`] and the [`Path`] where it was found. [`Validate::validate_fail_fast`] returns only the first issue,
//! which is the one `validate` would list first. Fields are reported under the names serde reads them by.
//!
//! With the `serde` feature, on by default, `#[derive(Deserialize)]` checks a struct while it is deserialized:
//!
//! - [`from_deserializer`] returns every missing field and broken rule together.
//! - serde's own `Deserialize` works too. For a struct that breaks its rules, it returns an error with every issue
//!   in its message.
//!
//! The `json` feature adds [`json::from_str`] and [`json::from_slice`], which read JSON this way.

#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "json")]
pub mod json;
pub mod rules;

#[doc(hidden)]
pub mod __private;

#[cfg(feature = "serde")]
mod collecting_type_errors;
#[cfg(feature = "serde")]
mod content;
#[cfg(feature = "serde")]
mod content_deserializer;
#[cfg(feature = "serde")]
mod deserialize;
#[cfg(feature = "serde")]
mod deserialize_containers;
#[cfg(feature = "serde")]
mod deserialize_error;
mod issue;
mod issues;
mod location;
mod param_value;
mod path;
mod problem;
mod report;
mod rule;
mod validate;

#[cfg(feature = "serde")]
pub use deserialize::{Deserialize, from_deserializer, from_deserializer_fail_fast};
#[cfg(feature = "serde")]
pub use deserialize_error::DeserializeError;
pub use location::Location;
pub use param_value::ParamValue;
pub use path::{Path, PathSegment};
pub use problem::{Bounds, LengthUnit, Problem};
pub use report::Report;
pub use rule::Rule;
pub use validate::Validate;

/// Implements [`Validate`] from the `#[validate(...)]` rules on a struct's fields.
///
/// ```
/// use annotated::Validate;
///
/// #[derive(Validate)]
/// #[serde(rename_all = "camelCase")]
/// struct NewMember {
///     #[validate(required, length(max = 50))]
///     full_name: String,
/// }
///
/// let member = NewMember { full_name: "  ".to_owned() };
///
/// assert_eq!(member.validate().unwrap_err().to_string(), "fullName: is required");
/// ```
///
/// # Writing rules
///
/// Each entry in `#[validate(...)]` is a rule, and a field's rules are checked in the order written.
///
/// - A single name is a built-in rule from [`rules`]: `required` becomes `annotated::rules::required()`.
/// - A named argument calls the builder method of that name: `length(max = 50)` becomes
///   `annotated::rules::length().max(50)`.
/// - `message = "..."` in a rule's arguments replaces its problem's message. `{param}` placeholders are filled from the
///   problem's parameters.
/// - `length` takes `unit = "chars"`, `"bytes"` or, with the `unicode` feature, `"graphemes"`. A unit on a collection
///   is a compile error.
/// - `pattern`, with the `regex` feature, takes its regular expression as a string literal. The expression is compiled
///   once, and one that doesn't parse is a compile error.
///
/// # How fields are checked
///
/// - Every field is checked. The first rule a field breaks stops that field's other rules. `#[validate(report_all)]` on
///   the struct reports every rule a field breaks instead.
/// - On an `Option` field, `required` checks the `Option` itself. Every other rule checks the value inside, and is
///   skipped when there is none.
/// - `nested` validates a field whose type implements [`Validate`], and reports under the field's name.
/// - `each(...)` applies its rules to every element of a collection, reported at its index, or to every value of a map,
///   reported at its key. It works on anything that implements [`rules::HasElements`]. `each(nested)` validates every
///   element.
/// - A field is reported under the name serde reads it by, from `#[serde(...)]` attributes such as `rename_all` and
///   `rename`. The struct doesn't need to derive serde's traits.
/// - [`Validate::validate_fail_fast`] checks the same rules in the same order, nested values and elements included, and
///   returns only the first issue, even under `report_all`. It stops right after that issue, with one
///   exception: when the issue is inside a `nested` value, or inside an element checked with `nested`, the struct
///   around it may check more before it stops. It records nothing more.
///
/// # Rules on the whole struct
///
/// `#[validate(custom(check))]` on the struct checks the whole struct with any `Fn(&Self) -> Result<(), Problem>`.
///
/// - It reports at the struct's own location: the root, or the field the struct is `nested` under.
///   `custom(check, field = name)` reports at that field instead, under its serde name.
/// - `message = "..."` replaces the problem's message.
/// - Struct-level rules are checked after every field, in the order written. They can share a `#[validate(...)]` with
///   `report_all`, or have one of their own.
/// - A rule with `field` counts as that field's last rule. It is skipped once the field has an issue, and once one
///   of them fails for a field, the later ones for that field are skipped. With `report_all` on the struct, none of
///   them is skipped.
/// - A rule without `field` always runs.
///
/// ```
/// use annotated::{Problem, Validate};
///
/// #[derive(Validate)]
/// #[serde(rename_all = "camelCase")]
/// #[validate(custom(passwords_match, field = repeated_password))]
/// struct NewPassword {
///     #[validate(length(min = 8))]
///     password: String,
///     repeated_password: String,
/// }
///
/// fn passwords_match(new_password: &NewPassword) -> Result<(), Problem> {
///     if new_password.repeated_password != new_password.password {
///         return Err(Problem::custom("passwords_differ", "must match the password"));
///     }
///     Ok(())
/// }
///
/// let new_password = NewPassword {
///     password: "correct horse battery staple".to_owned(),
///     repeated_password: "correct horse".to_owned(),
/// };
///
/// assert_eq!(new_password.validate().unwrap_err().to_string(), "repeatedPassword: must match the password");
/// ```
pub use annotated_derive::Validate;

/// Implements [`Deserialize`](trait@Deserialize), and serde's own `Deserialize` through it, for a struct with named
/// fields. Deserializing the struct reports every missing field and broken rule together.
///
/// ```
/// use annotated::Deserialize;
///
/// #[derive(Deserialize, Debug)]
/// #[serde(rename_all = "camelCase")]
/// struct NewMember {
///     #[validate(required, length(max = 50))]
///     full_name: String,
///     #[validate(email)]
///     email: String,
/// }
///
/// let mut body = serde_json::Deserializer::from_str(r#"{ "fullName": "  " }"#);
/// let error = annotated::from_deserializer::<NewMember, _>(&mut body).unwrap_err();
///
/// assert_eq!(error.to_string(), "fullName: is required\nemail: is missing");
/// ```
///
/// # Serde attributes
///
/// The struct is read as serde's derive would read it, by the names serde gives the struct and its fields.
///
/// - **Supported:** `rename_all`, `rename`, `deny_unknown_fields`, `expecting` and `default` on the struct, and
///   `rename`, `alias`, `default`, `skip_deserializing` and `skip` on a field.
/// - **Accepted and ignored:** attributes that only change serializing, so that the struct can derive serde's
///   `Serialize` too. These are `into` and `bound(serialize = "...")` on the struct, and `skip_serializing`,
///   `skip_serializing_if`, `serialize_with`, `getter` and `bound(serialize = "...")` on a field. When a `rename` or
///   `rename_all` gives separate names for serializing and deserializing, the deserializing name is the one read.
/// - **Compile errors:** any other serde attribute, and a struct with lifetime parameters.
///
/// # What ends deserialization
///
/// Unknown fields are ignored, unless the struct denies them. A field that appears twice, an unknown field the struct
/// denies, and a value of the wrong type are each the format's own error, which ends deserialization. A struct with
/// `#[annotated(collect_type_errors)]` reports values of the wrong type as issues instead.
///
/// # How fields are checked
///
/// Once the input is read to its end, every field is checked in the order the struct declares them:
///
/// - A field absent from the input is [`Problem::Missing`], at its name. An `Option` field is `None` instead, as with
///   serde, and `required` on it reports [`Problem::Required`].
/// - A field with `#[serde(default)]` or `#[serde(default = "path")]` takes its default when it is absent. With
///   either on the struct, an absent field without a default of its own takes its value in the struct's default.
///   Its rules check that value like a value read. A `nested` rule checks it with [`Validate`], so the nested type
///   must implement `Validate` too.
/// - A field with `#[serde(skip_deserializing)]` or `#[serde(skip)]` is never read. It takes its default, or
///   `Default::default()` when it has none, and its rules check that value the same way. A key with its name is an
///   unknown field.
/// - A field that is present is checked by its `#[validate(...)]` rules exactly as [`Validate`] checks it. So for input
///   that has every field, the issues are the same, in the same order, as when deserializing with serde and then
///   validating.
/// - A field checked with `nested`, or with `each(...)` holding `nested`, is read through
///   [`Deserialize`](trait@Deserialize):
///   - A field missing inside it is reported at its full path.
///   - The issues inside it are listed where its `nested` rule comes among the field's rules, as [`Validate`]
///     lists them.
///   - Its type must implement `annotated::Deserialize`. The containers that hold such values do: `Option`, `Box`,
///     sequences and maps among them.
///   - When its value couldn't be built, its issues are reported and its other rules are skipped.
///   - A field's rules can name `nested` only once.
/// - Struct-level rules run last, as with [`Validate`], once every field was present and could be built. Otherwise they
///   are skipped, since there is no struct to check.
///
/// ```
/// use annotated::Deserialize;
///
/// #[derive(Deserialize, Debug)]
/// #[serde(rename_all = "camelCase")]
/// struct Address {
///     #[validate(required)]
///     street: String,
///     #[validate(length(min = 5))]
///     postal_code: String,
/// }
///
/// #[derive(Deserialize, Debug)]
/// struct Route {
///     #[validate(each(nested))]
///     stops: Vec<Address>,
/// }
///
/// let body = r#"{ "stops": [{ "street": "Gran Vía 1" }, { "street": "", "postalCode": "28013" }] }"#;
/// let mut body = serde_json::Deserializer::from_str(body);
/// let error = annotated::from_deserializer::<Route, _>(&mut body).unwrap_err();
///
/// assert_eq!(error.to_string(), "stops[0].postalCode: is missing\nstops[1].street: is required");
/// ```
///
/// # Failing fast
///
/// [`from_deserializer_fail_fast`] returns only the first issue, which is the one [`from_deserializer`] would list
/// first. The input is still read to its end, because a field is known to be missing only then. Once it is read, the
/// struct stops checking its fields at the first issue. A nested value is checked while it is read, so its rules
/// may still run.
///
/// # Values of the wrong type
///
/// `#[annotated(collect_type_errors)]` on the struct turns a value of the wrong type into one more issue,
/// [`Problem::InvalidType`] at the field, and deserialization goes on. To make that possible, each field is read in two
/// steps: first into a buffer that holds any well-formed value, then from the buffer as the field's type.
///
/// - The `expected` parameter is what serde says the type expects (`u8`, `a string`, `struct Address`). When serde
///   doesn't say, as when the type rejects the value with a message of its own, it is the type's name.
/// - The `found` parameter is the kind of value in the input (`string`, `integer`, `null`, …), never the value itself.
/// - A value of the wrong type is checked by no other rule, and the struct-level rules are skipped, as when a field is
///   missing.
/// - A nested struct that collects type errors too reports the ones inside it at their full paths.
/// - The buffer borrows the strings and bytes the format lends.
/// - It needs a self-describing format, such as JSON.
///
/// ```
/// use annotated::Deserialize;
///
/// #[derive(Deserialize, Debug)]
/// #[annotated(collect_type_errors)]
/// struct NewMember {
///     #[validate(email)]
///     email: String,
///     #[validate(range(min = 18))]
///     age: u8,
/// }
///
/// let mut body = serde_json::Deserializer::from_str(r#"{ "email": "lucia", "age": "twelve" }"#);
/// let error = annotated::from_deserializer::<NewMember, _>(&mut body).unwrap_err();
///
/// assert_eq!(error.to_string(), "email: must be an email address\nage: must be u8, found string");
/// ```
#[cfg(feature = "serde")]
pub use annotated_derive::Deserialize;
pub use issue::Issue;
pub use issues::Issues;
