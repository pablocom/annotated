use proc_macro2::Span;
use serde_derive_internals::ast::{Container, Data};
use serde_derive_internals::{Ctxt, Derive};
use syn::{DeriveInput, Ident};

pub struct SerdeStruct {
    pub name: String,
    pub fields: Vec<SerdeField>,
    pub is_denying_unknown_fields: bool,
    pub expecting: Option<String>,
}

pub struct SerdeField {
    pub name: String,
    pub names_read: Vec<String>,
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
