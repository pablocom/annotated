use proc_macro2::Span;
use serde_derive_internals::ast::{Container, Data};
use serde_derive_internals::{Ctxt, Derive, attr};
use syn::{DeriveInput, ExprPath, Ident};

pub struct SerdeStruct {
    pub name: String,
    pub fields: Vec<SerdeField>,
    pub is_denying_unknown_fields: bool,
    pub expecting: Option<String>,
}

pub struct SerdeField {
    pub name: String,
    pub names_read: Vec<String>,
    pub default: Option<DefaultValue>,
}

pub enum DefaultValue {
    OfItsType,
    ReturnedBy(ExprPath),
}

pub fn field_names(input: &DeriveInput) -> syn::Result<Vec<String>> {
    let serde_struct = SerdeStruct::from_input(input)?;
    Ok(serde_struct.fields.into_iter().map(|field| field.name).collect())
}

impl SerdeStruct {
    pub fn from_input(input: &DeriveInput) -> syn::Result<Self> {
        let context = Ctxt::new();
        let serde_private_module = Ident::new("__private", Span::call_site());
        let serde_struct = Container::from_ast(&context, input, Derive::Deserialize, &serde_private_module)
            .map(|container| Self::from_container(&container));
        context.check()?;
        Ok(serde_struct.expect("serde reads any struct it finds no error in"))
    }

    fn from_container(container: &Container) -> Self {
        let fields = match &container.data {
            Data::Struct(_, fields) => fields
                .iter()
                .map(|field| SerdeField {
                    name: field.attrs.name().deserialize_name().value.clone(),
                    names_read: field.attrs.aliases().iter().map(|name| name.value.clone()).collect(),
                    default: DefaultValue::from_serde(field.attrs.default()),
                })
                .collect(),
            Data::Enum(_) => Vec::new(),
        };
        Self {
            name: container.attrs.name().deserialize_name().value.clone(),
            fields,
            is_denying_unknown_fields: container.attrs.deny_unknown_fields(),
            expecting: container.attrs.expecting().map(ToOwned::to_owned),
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
