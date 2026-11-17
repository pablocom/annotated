use proc_macro2::Span;
use serde_derive_internals::ast::{self, Container, Data};
use serde_derive_internals::{Ctxt, Derive, attr};
use syn::punctuated::Punctuated;
use syn::{Attribute, DeriveInput, ExprPath, Ident, Lifetime, Meta, Path, Token, WherePredicate, parse_quote};

pub struct SerdeStruct {
    pub name: String,
    pub fields: Vec<SerdeField>,
    pub is_denying_unknown_fields: bool,
    pub is_transparent: bool,
    pub expecting: Option<String>,
    pub default: Option<DefaultValue>,
    pub borrowed_lifetimes: Vec<Lifetime>,
    pub bound: Option<Vec<WherePredicate>>,
    pub variant_bound: Option<Vec<WherePredicate>>,
    pub serde_path: Option<Path>,
}

pub struct SerdeField {
    pub name: String,
    pub names_read: Vec<String>,
    pub is_skipped: bool,
    pub is_flattened: bool,
    pub default: Option<DefaultValue>,
    pub deserialize_with: Option<ExprPath>,
    pub bound: Option<Vec<WherePredicate>>,
}

pub struct SerdeEnum {
    pub name: String,
    pub tag: Tag,
    pub variants: Vec<SerdeVariant>,
    pub is_denying_unknown_fields: bool,
    pub expecting: Option<String>,
    pub borrowed_lifetimes: Vec<Lifetime>,
    pub bound: Option<Vec<WherePredicate>>,
    pub serde_path: Option<Path>,
}

pub enum Tag {
    External,
    Internal { tag: String },
    Adjacent { tag: String, content: String },
    Untagged,
}

pub struct SerdeVariant {
    pub name: String,
    pub names_read: Vec<String>,
    pub is_untagged: bool,
    pub is_skipped: bool,
    pub is_other: bool,
    pub deserialize_with: Option<ExprPath>,
    pub bound: Option<Vec<WherePredicate>>,
    pub fields: Vec<SerdeField>,
}

pub enum DefaultValue {
    OfItsType,
    ReturnedBy(ExprPath),
}

// What reading serde's attributes does with one that serde's own derive would refuse, because `serde_derive_internals`
// doesn't know it, as one added by a later serde, or because it is malformed.
#[derive(Clone, Copy)]
pub enum UnreadableAttribute {
    // `Deserialize` reads the input as serde would, so it can't go on without the attribute.
    IsAnError,
    // `Validate` needs serde's names only, and serde's own derive, when the type has it, reports the attribute.
    // `serde_derive_internals` stops reading a `#[serde(...)]` at its first error, so the attributes are then read
    // again one by one, so that the one it can't read takes no other with it.
    IsLeftOut,
}

fn read<T>(input: &DeriveInput, unreadable: UnreadableAttribute, reading: fn(&Container) -> T) -> syn::Result<T> {
    let context = Ctxt::new();
    let read = read_with(&context, input, reading);
    match (context.check(), unreadable) {
        (Ok(()), _) => Ok(read),
        (Err(error), UnreadableAttribute::IsAnError) => Err(error),
        (Err(_), UnreadableAttribute::IsLeftOut) => {
            let context = Ctxt::new();
            let read = read_with(&context, &with_serde_attributes_one_by_one(input), reading);
            drop(context.check());
            Ok(read)
        }
    }
}

fn read_with<T>(context: &Ctxt, input: &DeriveInput, reading: fn(&Container) -> T) -> T {
    let serde_private_module = Ident::new("__private", Span::call_site());
    let container = Container::from_ast(context, input, Derive::Deserialize, &serde_private_module)
        .expect("serde reads a struct or an enum, whatever its attributes");
    reading(&container)
}

fn with_serde_attributes_one_by_one(input: &DeriveInput) -> DeriveInput {
    let mut input = input.clone();
    one_by_one(&mut input.attrs);
    match &mut input.data {
        syn::Data::Struct(data) => data.fields.iter_mut().for_each(|field| one_by_one(&mut field.attrs)),
        syn::Data::Enum(data) => {
            for variant in &mut data.variants {
                one_by_one(&mut variant.attrs);
                variant.fields.iter_mut().for_each(|field| one_by_one(&mut field.attrs));
            }
        }
        syn::Data::Union(_) => {}
    }
    input
}

// An attribute that doesn't parse as a list of serde's attributes is left out whole.
fn one_by_one(attributes: &mut Vec<Attribute>) {
    *attributes = attributes
        .drain(..)
        .flat_map(|attribute| {
            if !attribute.path().is_ident("serde") {
                return vec![attribute];
            }
            attribute
                .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
                .map_or_else(
                    |_| Vec::new(),
                    |serde_attributes| {
                        serde_attributes
                            .into_iter()
                            .map(|serde_attribute| parse_quote!(#[serde(#serde_attribute)]))
                            .collect()
                    },
                )
        })
        .collect();
}

impl SerdeStruct {
    pub fn from_input(input: &DeriveInput, unreadable: UnreadableAttribute) -> syn::Result<Self> {
        read(input, unreadable, Self::from_container)
    }

    fn from_container(container: &Container) -> Self {
        let fields = match &container.data {
            Data::Struct(_, fields) => fields.iter().map(SerdeField::from_serde).collect(),
            Data::Enum(_) => Vec::new(),
        };
        Self {
            name: container.attrs.name().deserialize_name().value.clone(),
            fields,
            is_denying_unknown_fields: container.attrs.deny_unknown_fields(),
            is_transparent: container.attrs.transparent(),
            expecting: container.attrs.expecting().map(ToOwned::to_owned),
            default: DefaultValue::from_serde(container.attrs.default()),
            borrowed_lifetimes: borrowed_lifetimes(container),
            bound: container.attrs.de_bound().map(<[WherePredicate]>::to_vec),
            variant_bound: None,
            serde_path: container.attrs.custom_serde_path().cloned(),
        }
    }
}

impl SerdeEnum {
    pub fn from_input(input: &DeriveInput, unreadable: UnreadableAttribute) -> syn::Result<Self> {
        read(input, unreadable, Self::from_container)
    }

    fn from_container(container: &Container) -> Self {
        let tag = match container.attrs.tag() {
            attr::TagType::External => Tag::External,
            attr::TagType::Internal { tag } => Tag::Internal { tag: tag.clone() },
            attr::TagType::Adjacent { tag, content } => Tag::Adjacent {
                tag: tag.clone(),
                content: content.clone(),
            },
            attr::TagType::None => Tag::Untagged,
        };
        let variants = match &container.data {
            Data::Enum(variants) => variants
                .iter()
                .map(|variant| SerdeVariant {
                    name: variant.attrs.name().deserialize_name().value.clone(),
                    names_read: variant.attrs.aliases().iter().map(|name| name.value.clone()).collect(),
                    is_untagged: variant.attrs.untagged(),
                    is_skipped: variant.attrs.skip_deserializing(),
                    is_other: variant.attrs.other(),
                    deserialize_with: variant.attrs.deserialize_with().map(reachable_function),
                    bound: variant.attrs.de_bound().map(<[WherePredicate]>::to_vec),
                    fields: variant.fields.iter().map(SerdeField::from_serde).collect(),
                })
                .collect(),
            Data::Struct(..) => Vec::new(),
        };
        Self {
            name: container.attrs.name().deserialize_name().value.clone(),
            tag,
            variants,
            is_denying_unknown_fields: container.attrs.deny_unknown_fields(),
            expecting: container.attrs.expecting().map(ToOwned::to_owned),
            borrowed_lifetimes: borrowed_lifetimes(container),
            bound: container.attrs.de_bound().map(<[WherePredicate]>::to_vec),
            serde_path: container.attrs.custom_serde_path().cloned(),
        }
    }
}

impl SerdeField {
    fn from_serde(field: &ast::Field) -> Self {
        Self {
            name: field.attrs.name().deserialize_name().value.clone(),
            names_read: field.attrs.aliases().iter().map(|name| name.value.clone()).collect(),
            is_skipped: field.attrs.skip_deserializing(),
            is_flattened: field.attrs.flatten(),
            default: DefaultValue::from_serde(field.attrs.default()),
            deserialize_with: field.attrs.deserialize_with().map(reachable_function),
            bound: field.attrs.de_bound().map(<[WherePredicate]>::to_vec),
        }
    }
}

impl DefaultValue {
    fn from_serde(default: &attr::Default) -> Option<Self> {
        match default {
            attr::Default::None => None,
            attr::Default::Default => Some(Self::OfItsType),
            attr::Default::Path(function) => Some(Self::ReturnedBy(function.clone())),
        }
    }
}

// As serde's derive works it out: `'de` outlives every lifetime a field that is read borrows.
fn borrowed_lifetimes(container: &Container) -> Vec<Lifetime> {
    let mut lifetimes: Vec<Lifetime> = container
        .data
        .all_fields()
        .filter(|field| !field.attrs.skip_deserializing())
        .flat_map(|field| field.attrs.borrowed_lifetimes().iter().cloned())
        .collect();
    lifetimes.sort();
    lifetimes.dedup();
    lifetimes
}

// For a `Cow<str>` or `Cow<[u8]>` field with `#[serde(borrow)]`, `serde_derive_internals` names a function in serde's
// private module, which the generated code can't reach. annotated has its own.
fn reachable_function(function: &ExprPath) -> ExprPath {
    let segments = &function.path.segments;
    let is_in_serde_s_private_module = segments.first().is_some_and(|first| first.ident == "_serde");
    if !is_in_serde_s_private_module {
        return function.clone();
    }
    let name = &segments.last().expect("a path has a segment").ident;
    parse_quote!(_annotated::__private::#name)
}
