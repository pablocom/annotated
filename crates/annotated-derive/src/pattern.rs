use proc_macro2::{Span, TokenStream};
use quote::quote_spanned;
use syn::{Error, Expr, ExprLit, Lit, LitStr};

use crate::written_rule::WrittenRule;

pub fn pattern_rule(written: &WrittenRule, span: Span) -> syn::Result<TokenStream> {
    let expression = regular_expression(written, span)?;
    check_regular_expression(expression, span)?;
    Ok(quote_spanned! {span=>
        {
            static PATTERN: ::std::sync::LazyLock<::annotated::__private::Regex> =
                ::std::sync::LazyLock::new(|| {
                    ::annotated::__private::Regex::new(#expression)
                        .expect("`#[derive(Validate)]` parsed this regular expression")
                });
            ::annotated::rules::pattern(&PATTERN)
        }
    })
}

fn regular_expression(written: &WrittenRule, span: Span) -> syn::Result<&LitStr> {
    match &written.value {
        Some(Expr::Lit(ExprLit {
            lit: Lit::Str(expression),
            ..
        })) if written.builder_arguments.is_empty() => Ok(expression),
        _ => Err(Error::new(
            span,
            r#"`pattern` takes its regular expression as a string literal: `pattern = r"..."`"#,
        )),
    }
}

#[cfg(feature = "regex")]
fn check_regular_expression(expression: &LitStr, _: Span) -> syn::Result<()> {
    regex_syntax::Parser::new()
        .parse(&expression.value())
        .map(drop)
        .map_err(|error| {
            Error::new_spanned(
                expression,
                format!("`pattern` can't parse this regular expression:\n{error}"),
            )
        })
}

#[cfg(not(feature = "regex"))]
fn check_regular_expression(_: &LitStr, span: Span) -> syn::Result<()> {
    Err(Error::new(
        span,
        "the `pattern` rule needs the `regex` feature of `annotated`",
    ))
}
