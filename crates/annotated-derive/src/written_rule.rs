use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Attribute, Error, Expr, Ident, Path, Token, parenthesized, token};

pub struct WrittenRule {
    pub path: Path,
    pub value: Option<Expr>,
    pub builder_arguments: Vec<NamedArgument>,
    pub message: Option<Expr>,
}

pub struct NamedArgument {
    pub name: Ident,
    pub value: Expr,
}

enum Argument {
    Named(NamedArgument),
    Value(Expr),
}

pub fn written_rules(attributes: &[Attribute]) -> syn::Result<Vec<WrittenRule>> {
    let mut rules = Vec::new();
    for attribute in validate_attributes(attributes) {
        rules.extend(attribute.parse_args_with(Punctuated::<WrittenRule, Token![,]>::parse_terminated)?);
    }
    Ok(rules)
}

pub fn validate_attributes(attributes: &[Attribute]) -> impl Iterator<Item = &Attribute> {
    attributes
        .iter()
        .filter(|attribute| attribute.path().is_ident("validate"))
}

impl WrittenRule {
    fn without_arguments(path: Path) -> Self {
        Self {
            path,
            value: None,
            builder_arguments: Vec::new(),
            message: None,
        }
    }

    fn from_arguments(path: Path, arguments: impl IntoIterator<Item = Argument>) -> syn::Result<Self> {
        let mut rule = Self::without_arguments(path);
        for argument in arguments {
            match argument {
                Argument::Named(named) if named.name == "message" => rule.message = Some(named.value),
                Argument::Named(named) => rule.builder_arguments.push(named),
                Argument::Value(value) if rule.value.is_some() => {
                    return Err(Error::new_spanned(value, "a rule takes one value at most"));
                }
                Argument::Value(value) => rule.value = Some(value),
            }
        }
        if let (Some(_), Some(builder_argument)) = (&rule.value, rule.builder_arguments.first()) {
            return Err(Error::new_spanned(
                &builder_argument.name,
                "a rule takes either one value or named arguments, not both",
            ));
        }
        Ok(rule)
    }
}

impl Parse for WrittenRule {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path = input.call(Path::parse_mod_style)?;
        if input.peek(Token![=]) {
            input.parse::<Token![=]>()?;
            return Ok(Self {
                value: Some(input.parse()?),
                ..Self::without_arguments(path)
            });
        }
        if !input.peek(token::Paren) {
            return Ok(Self::without_arguments(path));
        }
        let arguments;
        parenthesized!(arguments in input);
        let arguments = Punctuated::<Argument, Token![,]>::parse_terminated(&arguments)?;
        Self::from_arguments(path, arguments)
    }
}

impl Parse for Argument {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let is_named = input.peek(Ident) && input.peek2(Token![=]) && !input.peek2(Token![==]);
        if !is_named {
            return Ok(Self::Value(input.parse()?));
        }
        let name = input.parse()?;
        input.parse::<Token![=]>()?;
        let value = input.parse()?;
        Ok(Self::Named(NamedArgument { name, value }))
    }
}
