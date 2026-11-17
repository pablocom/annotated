use proc_macro2::{Span, TokenStream};
use quote::quote_spanned;
use syn::{Error, Expr, ExprLit, Lit, LitStr};

use crate::written_rule::RuleCall;

pub fn pattern_rule(call: &RuleCall, span: Span) -> syn::Result<TokenStream> {
    let expression = regular_expression(call, span)?;
    check_regular_expression(expression, span)?;
    Ok(quote_spanned! {span=>
        {
            static PATTERN: ::std::sync::LazyLock<::annotated::__private::Regex> =
                ::std::sync::LazyLock::new(|| {
                    ::annotated::__private::Regex::new(#expression)
                        .expect("`#[derive(Validate)]` built this regular expression")
                });
            ::annotated::rules::pattern(&PATTERN)
        }
    })
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
