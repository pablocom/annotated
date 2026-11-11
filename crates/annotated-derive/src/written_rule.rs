use proc_macro2::Span;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Attribute, Error, Expr, Ident, Path, Token, parenthesized, token};

pub enum WrittenRule {
    Each {
        span: Span,
        element_rules: Vec<WrittenRule>,
    },
    Call(Box<RuleCall>),
}

pub struct RuleCall {
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

impl Parse for WrittenRule {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path = input.call(Path::parse_mod_style)?;
        if path.is_ident("each") {
            return Ok(Self::Each {
                span: path.span(),
                element_rules: element_rules(&path, input)?,
            });
        }
        Ok(Self::Call(Box::new(RuleCall::parse_after(path, input)?)))
    }
}

fn element_rules(each: &Path, input: ParseStream) -> syn::Result<Vec<WrittenRule>> {
    if !input.peek(token::Paren) {
        return Err(Error::new_spanned(
            each,
            "`each` takes the rules for every element in parentheses: `each(...)`",
        ));
    }
    let element_rules;
    parenthesized!(element_rules in input);
    let element_rules = Punctuated::<WrittenRule, Token![,]>::parse_terminated(&element_rules)?;
    for element_rule in &element_rules {
        if let WrittenRule::Call(call) = element_rule
            && call.path.is_ident("message")
        {
            return Err(Error::new_spanned(
                &call.path,
                "`message` goes on the rules inside `each`, not on `each` itself",
            ));
        }
    }
    Ok(element_rules.into_iter().collect())
}

impl RuleCall {
    fn parse_after(path: Path, input: ParseStream) -> syn::Result<Self> {
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

    pub fn has_arguments(&self) -> bool {
        self.value.is_some() || !self.builder_arguments.is_empty() || self.message.is_some()
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
