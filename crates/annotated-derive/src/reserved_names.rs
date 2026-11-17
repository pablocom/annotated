use proc_macro2::{TokenStream, TokenTree};
use quote::ToTokens;
use syn::{Attribute, Data, DeriveInput, Error, Fields, Ident};

// The names the code a derive generates gives items and parameters of its own, in the scope the user's tokens are in:
// a field's type, a bound or a rule's arguments naming one would name the derive's instead. Locals and labels are
// hygienic, so the user's can't meet them.
pub const OF_VALIDATE: ReservedNames = ReservedNames {
    derive: "Validate",
    names: &["_annotated"],
    prefixes: &["__ANNOTATED_PATTERN_"],
    lifetimes: &[],
};

pub const OF_DESERIALIZE: ReservedNames = ReservedNames {
    derive: "Deserialize",
    names: &[
        "_annotated",
        "_serde",
        "__Visitor",
        "__Field",
        "__FieldVisitor",
        "__FIELDS",
        "__Variant",
        "__VariantVisitor",
        "__VARIANTS",
        "__EnumVisitor",
        "__VariantSeed",
        "__AdjacentVisitor",
        "__ADJACENT_FIELDS",
        "__DeserializeVariantWith",
        "__D",
        "__E",
        "__A",
    ],
    prefixes: &["__ANNOTATED_PATTERN_", "__DeserializeWith"],
    lifetimes: &["de", "__visitor"],
};

pub struct ReservedNames {
    derive: &'static str,
    names: &'static [&'static str],
    prefixes: &'static [&'static str],
    lifetimes: &'static [&'static str],
}

impl ReservedNames {
    // As serde's derive refuses a lifetime parameter named `'de`, so that the user's name never quietly means the
    // derive's item. A field's or a variant's name is no item's, so it can be any.
    pub fn refuse_those_in(&self, input: &DeriveInput) -> syn::Result<()> {
        let attributes = |attributes: &[Attribute]| attributes.iter().map(ToTokens::to_token_stream).collect();
        let fields = |fields: &Fields| -> TokenStream {
            fields
                .iter()
                .flat_map(|field| [attributes(&field.attrs), field.ty.to_token_stream()])
                .collect()
        };
        let data: TokenStream = match &input.data {
            Data::Struct(data) => fields(&data.fields),
            Data::Enum(data) => data
                .variants
                .iter()
                .flat_map(|variant| [attributes(&variant.attrs), fields(&variant.fields)])
                .collect(),
            Data::Union(_) => TokenStream::new(),
        };
        self.refuse_those_among(input.ident.to_token_stream())?;
        self.refuse_those_among(input.generics.to_token_stream())?;
        self.refuse_those_among(input.generics.where_clause.to_token_stream())?;
        self.refuse_those_among(attributes(&input.attrs))?;
        self.refuse_those_among(data)
    }

    fn refuse_those_among(&self, tokens: TokenStream) -> syn::Result<()> {
        let mut follows_an_apostrophe = false;
        for token in tokens {
            match token {
                TokenTree::Punct(punct) if punct.as_char() == '\'' => {
                    follows_an_apostrophe = true;
                    continue;
                }
                TokenTree::Ident(ident) => {
                    if let Some(refusal) = self.refusal_of(&ident, follows_an_apostrophe) {
                        return Err(Error::new(ident.span(), refusal));
                    }
                }
                TokenTree::Group(group) => self.refuse_those_among(group.stream())?,
                TokenTree::Punct(_) | TokenTree::Literal(_) => {}
            }
            follows_an_apostrophe = false;
        }
        Ok(())
    }

    fn refusal_of(&self, ident: &Ident, is_a_lifetime: bool) -> Option<String> {
        let derive = self.derive;
        let name = ident.to_string();
        if is_a_lifetime {
            return self.lifetimes.contains(&name.as_str()).then(|| {
                format!("`'{name}` is a lifetime `#[derive({derive})]` gives its impls: give yours another name")
            });
        }
        if self.names.contains(&name.as_str()) {
            return Some(format!(
                "`{name}` is a name `#[derive({derive})]` gives an item or a parameter of its own: give yours \
                 another name"
            ));
        }
        let prefix = self.prefixes.iter().find(|prefix| name.starts_with(**prefix))?;
        Some(format!(
            "`{name}` begins with `{prefix}`, as the names `#[derive({derive})]` gives items of its own do: give \
             yours another name"
        ))
    }
}
