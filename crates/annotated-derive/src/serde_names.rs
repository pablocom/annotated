use proc_macro2::Span;
use serde_derive_internals::ast::{self, Container, Data};
use serde_derive_internals::{Ctxt, Derive, attr};
use syn::{DeriveInput, ExprPath, Ident, Lifetime, parse_quote};

pub struct SerdeStruct {
    pub name: String,
    pub fields: Vec<SerdeField>,
    pub is_denying_unknown_fields: bool,
    pub expecting: Option<String>,
    pub default: Option<DefaultValue>,
    pub borrowed_lifetimes: Vec<Lifetime>,
}

pub struct SerdeField {
    pub name: String,
    pub names_read: Vec<String>,
    pub is_skipped: bool,
    pub is_flattened: bool,
    pub default: Option<DefaultValue>,
    pub deserialize_with: Option<ExprPath>,
}

pub struct SerdeEnum {
    pub name: String,
    pub tag: Tag,
    pub variants: Vec<SerdeVariant>,
    pub is_denying_unknown_fields: bool,
    pub expecting: Option<String>,
    pub borrowed_lifetimes: Vec<Lifetime>,
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
    pub fields: Vec<SerdeField>,
}

pub enum DefaultValue {
    OfItsType,
    ReturnedBy(ExprPath),
}

fn read<T>(input: &DeriveInput, reading: impl FnOnce(&Container) -> T) -> syn::Result<T> {
    let context = Ctxt::new();
    let serde_private_module = Ident::new("__private", Span::call_site());
    let read = Container::from_ast(&context, input, Derive::Deserialize, &serde_private_module)
        .map(|container| reading(&container));
    context.check()?;
    Ok(read.expect("serde reads any item it finds no error in"))
}

impl SerdeStruct {
    pub fn from_input(input: &DeriveInput) -> syn::Result<Self> {
        read(input, Self::from_container)
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
            expecting: container.attrs.expecting().map(ToOwned::to_owned),
            default: DefaultValue::from_serde(container.attrs.default()),
            borrowed_lifetimes: borrowed_lifetimes(container),
        }
    }
}

impl SerdeEnum {
    pub fn from_input(input: &DeriveInput) -> syn::Result<Self> {
        read(input, Self::from_container)
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
    parse_quote!(::annotated::__private::#name)
}
