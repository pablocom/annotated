use std::hash::{DefaultHasher, Hash, Hasher};

use proc_macro2::{Span, TokenStream};
use quote::format_ident;
use syn::{Error, Expr, ExprLit, Ident, Lit, LitStr};

use crate::written_rule::RuleCall;

// A regular expression a `pattern` rule matches, compiled once into a `static` of the derive's own. Every rule with the
// same expression, the check and the rules as data alike, shares that `static`, so the expression is compiled once
// whatever reads it. A `static` is named after the expression, so that the rules name it as they are parsed: two
// expressions given the same name would be two `static`s of one name, a compile error, never one expression for the
// other.
pub struct Pattern {
    pub compiled: Ident,
    expression: LitStr,
    span: Span,
}

impl Pattern {
    pub fn of(call: &RuleCall, span: Span) -> syn::Result<Self> {
        let expression = regular_expression(call, span)?;
        check_regular_expression(expression, span)?;
        let mut hasher = DefaultHasher::new();
        expression.value().hash(&mut hasher);
        Ok(Self {
            compiled: format_ident!("__ANNOTATED_PATTERN_{:016X}", hasher.finish()),
            expression: expression.clone(),
            span,
        })
    }

    pub fn rule(&self, span: Span) -> TokenStream {
        let compiled = &self.compiled;
        quote_at!(span=> _annotated::rules::pattern(&#compiled))
    }

    pub fn is_the_same_as(&self, other: &Self) -> bool {
        self.expression.value() == other.expression.value()
    }

    pub fn compiled_once(&self) -> TokenStream {
        let Self {
            compiled,
            expression,
            span,
        } = self;
        quote_at! {*span=>
            static #compiled: ::std::sync::LazyLock<_annotated::__private::Regex> =
                ::std::sync::LazyLock::new(|| {
                    _annotated::__private::Regex::new(#expression)
                        .expect("`#[derive(Validate)]` built this regular expression")
                });
        }
    }
}

fn regular_expression(call: &RuleCall, span: Span) -> syn::Result<&LitStr> {
    match &call.value {
        Some(Expr::Lit(ExprLit {
            lit: Lit::Str(expression),
            ..
        })) if call.builder_arguments.is_empty() => Ok(expression),
        _ => Err(Error::new(
            span,
            r#"`pattern` takes its regular expression as a string literal: `pattern = r"..."`"#,
        )),
    }
}

#[cfg(feature = "regex")]
fn check_regular_expression(expression: &LitStr, _: Span) -> syn::Result<()> {
    regex::Regex::new(&expression.value())
        .map(drop)
        .map_err(|error| Error::new_spanned(expression, refusal_message(&error)))
}

#[cfg(feature = "regex")]
fn refusal_message(error: &regex::Error) -> String {
    match error {
        regex::Error::CompiledTooBig(size_limit) => format!(
            "`pattern` is too big: the `regex` crate refuses an expression that compiles to more than {size_limit} bytes.\nUse a smaller repetition or a simpler class."
        ),
        other => format!("`pattern` can't parse this regular expression:\n{other}"),
    }
}

#[cfg(not(feature = "regex"))]
fn check_regular_expression(_: &LitStr, span: Span) -> syn::Result<()> {
    Err(Error::new(
        span,
        "the `pattern` rule needs the `regex` feature of `annotated`",
    ))
}
