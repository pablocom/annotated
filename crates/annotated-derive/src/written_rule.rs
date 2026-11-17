use proc_macro2::Span;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Attribute, Error, Expr, ExprLit, ExprUnary, Ident, Lit, Path, Token, UnOp, parenthesized, token};

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

pub enum StructRule {
    ReportAll,
    Custom(Box<StructCustom>),
}

pub struct StructCustom {
    pub check: Expr,
    pub field: Option<Expr>,
    pub message: Option<Expr>,
    pub span: Span,
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

pub fn written_struct_rules(attributes: &[Attribute]) -> syn::Result<Vec<StructRule>> {
    whole_value_rules(attributes, StructRule::parse_on_a_struct)
}

pub fn written_enum_rules(attributes: &[Attribute]) -> syn::Result<Vec<StructRule>> {
    whole_value_rules(attributes, StructRule::parse_on_an_enum)
}

pub fn written_variant_rules(attributes: &[Attribute]) -> syn::Result<Vec<StructCustom>> {
    whole_value_rules(attributes, StructCustom::parse_on_a_variant)
}

fn whole_value_rules<R>(attributes: &[Attribute], parse: fn(ParseStream) -> syn::Result<R>) -> syn::Result<Vec<R>> {
    let mut rules = Vec::new();
    for attribute in validate_attributes(attributes) {
        rules.extend(
            attribute.parse_args_with(|input: ParseStream| {
                Punctuated::<R, Token![,]>::parse_terminated_with(input, parse)
            })?,
        );
    }
    Ok(rules)
}

fn validate_attributes(attributes: &[Attribute]) -> impl Iterator<Item = &Attribute> {
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

struct WholeValueRules {
    refusal: &'static str,
    custom_arguments: &'static str,
    field_refusal: Option<&'static str>,
}

const ON_A_STRUCT: WholeValueRules = WholeValueRules {
    refusal: "a struct's `#[validate(...)]` takes `report_all` and `custom(...)`",
    custom_arguments: "`custom` on a struct takes one function checking the struct, then optionally `field = ...` \
                       and `message = ...`: `custom(check, field = name)`",
    field_refusal: None,
};

const ON_AN_ENUM: WholeValueRules = WholeValueRules {
    refusal: "an enum's `#[validate(...)]` takes `report_all` and `custom(...)`",
    custom_arguments: "`custom` on an enum takes one function checking the enum, then optionally `message = ...`: \
                       `custom(check)`",
    field_refusal: Some(
        "`custom` on an enum takes no `field`: each field belongs to one variant, so write the rule on that variant",
    ),
};

const ON_A_VARIANT: WholeValueRules = WholeValueRules {
    refusal: "a variant's `#[validate(...)]` takes `custom(...)`: its fields take the other rules, and the enum \
              takes `report_all`",
    custom_arguments: "`custom` on a variant takes one function checking the enum, then optionally `field = ...` \
                       and `message = ...`: `custom(check, field = name)`",
    field_refusal: None,
};

impl StructRule {
    fn parse_on_a_struct(input: ParseStream) -> syn::Result<Self> {
        Self::parse_on(input, &ON_A_STRUCT)
    }

    fn parse_on_an_enum(input: ParseStream) -> syn::Result<Self> {
        Self::parse_on(input, &ON_AN_ENUM)
    }

    fn parse_on(input: ParseStream, whole_value_rules: &WholeValueRules) -> syn::Result<Self> {
        let path = input.call(Path::parse_mod_style)?;
        if path.is_ident("report_all") {
            return Ok(Self::ReportAll);
        }
        if !path.is_ident("custom") {
            return Err(Error::new_spanned(path, whole_value_rules.refusal));
        }
        Ok(Self::Custom(Box::new(StructCustom::parse_after(
            &path,
            input,
            whole_value_rules,
        )?)))
    }
}

impl StructCustom {
    fn parse_on_a_variant(input: ParseStream) -> syn::Result<Self> {
        let path = input.call(Path::parse_mod_style)?;
        if !path.is_ident("custom") {
            return Err(Error::new_spanned(path, ON_A_VARIANT.refusal));
        }
        Self::parse_after(&path, input, &ON_A_VARIANT)
    }

    fn parse_after(custom: &Path, input: ParseStream, whole_value_rules: &WholeValueRules) -> syn::Result<Self> {
        let refusal = whole_value_rules.custom_arguments;
        if !input.peek(token::Paren) {
            return Err(Error::new_spanned(custom, refusal));
        }
        let arguments;
        parenthesized!(arguments in input);
        let mut check = None;
        let mut field = None;
        let mut message = None;
        for argument in Punctuated::<Argument, Token![,]>::parse_terminated(&arguments)? {
            match argument {
                Argument::Value(value) if check.is_none() => check = Some(value),
                Argument::Named(named) if named.name == "field" => match whole_value_rules.field_refusal {
                    Some(field_refusal) => return Err(Error::new_spanned(named.name, field_refusal)),
                    None => given_once(&mut field, named)?,
                },
                Argument::Named(named) if named.name == "message" => given_once(&mut message, named)?,
                Argument::Value(value) => return Err(Error::new_spanned(value, refusal)),
                Argument::Named(named) => return Err(Error::new_spanned(named.name, refusal)),
            }
        }
        let check = check.ok_or_else(|| Error::new_spanned(custom, refusal))?;
        Ok(Self {
            check,
            field,
            message,
            span: custom.span(),
        })
    }
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
                Argument::Named(named) if named.name == "message" => given_once(&mut rule.message, named)?,
                Argument::Named(named) if rule.named_argument(&named.name.to_string()).is_some() => {
                    return Err(given_twice(&named.name));
                }
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
        rule.refuse_bounds_that_contradict()?;
        Ok(rule)
    }

    fn named_argument(&self, name: &str) -> Option<&NamedArgument> {
        self.builder_arguments.iter().find(|argument| argument.name == name)
    }

    // A built-in rule whose bounds contradict each other refuses every value. Bounds can be compared only when they are
    // written as literals: `min = SHORTEST` is left to the rule.
    fn refuse_bounds_that_contradict(&self) -> syn::Result<()> {
        let Some(rule_name) = ["length", "range"].into_iter().find(|name| self.path.is_ident(name)) else {
            return Ok(());
        };
        let equal = self.named_argument("equal").filter(|_| rule_name == "length");
        if let Some(equal) = equal
            && (self.named_argument("min").is_some() || self.named_argument("max").is_some())
        {
            return Err(Error::new_spanned(
                &equal.name,
                "`equal` is both bounds of `length`, so it takes no `min` or `max` beside it",
            ));
        }
        let (Some(min), Some(max)) = (self.named_argument("min"), self.named_argument("max")) else {
            return Ok(());
        };
        if is_greater(&min.value, &max.value) {
            return Err(Error::new_spanned(
                &min.name,
                format!("`min` is greater than `max`, so `{rule_name}` would refuse every value"),
            ));
        }
        Ok(())
    }

    pub fn has_arguments(&self) -> bool {
        self.value.is_some() || !self.builder_arguments.is_empty() || self.message.is_some()
    }
}

fn given_once(given: &mut Option<Expr>, argument: NamedArgument) -> syn::Result<()> {
    if given.is_some() {
        return Err(given_twice(&argument.name));
    }
    *given = Some(argument.value);
    Ok(())
}

fn given_twice(name: &Ident) -> Error {
    Error::new_spanned(
        name,
        format!("`{name}` is given twice: a rule takes each argument once"),
    )
}

fn is_greater(min: &Expr, max: &Expr) -> bool {
    match (number_written(min), number_written(max)) {
        (Some(Number::Integer(min)), Some(Number::Integer(max))) => min > max,
        (Some(min), Some(max)) => min.as_float() > max.as_float(),
        _ => false,
    }
}

#[derive(Clone, Copy)]
enum Number {
    Integer(i128),
    Float(f64),
}

impl Number {
    #[allow(
        clippy::cast_precision_loss,
        reason = "a bound past 2^53 is compared as near as a float can"
    )]
    const fn as_float(self) -> f64 {
        match self {
            Self::Integer(integer) => integer as f64,
            Self::Float(float) => float,
        }
    }
}

fn number_written(value: &Expr) -> Option<Number> {
    match value {
        Expr::Lit(ExprLit {
            lit: Lit::Int(integer), ..
        }) => integer.base10_parse().ok().map(Number::Integer),
        Expr::Lit(ExprLit {
            lit: Lit::Float(float), ..
        }) => float.base10_parse().ok().map(Number::Float),
        Expr::Unary(ExprUnary {
            op: UnOp::Neg(_), expr, ..
        }) => match number_written(expr)? {
            Number::Integer(integer) => Some(Number::Integer(-integer)),
            Number::Float(float) => Some(Number::Float(-float)),
        },
        Expr::Group(group) => number_written(&group.expr),
        Expr::Paren(parenthesized) => number_written(&parenthesized.expr),
        _ => None,
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
