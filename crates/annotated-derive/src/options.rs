use syn::punctuated::Punctuated;
use syn::{Attribute, Data, DeriveInput, Error, Expr, ExprLit, Lit, Meta, MetaNameValue, Path, Token};

// What `#[annotated(...)]` on the struct or the enum says. Both derives read it.
pub struct Options {
    pub collects_type_errors: bool,
    pub crate_path: Option<Path>,
}

const ACCEPTED: &str = "`#[annotated(...)]` takes `collect_type_errors` and `crate = \"...\"`";

const CRATE_PATH: &str = "`crate` takes the path annotated is reached by, as a string: `crate = \"my_api::annotated\"`";

impl Options {
    pub fn of(input: &DeriveInput) -> syn::Result<Self> {
        refuse_options_inside(input)?;
        let mut options = Self {
            collects_type_errors: false,
            crate_path: None,
        };
        for option in options_in(&input.attrs)? {
            match option {
                Meta::Path(path) if path.is_ident("collect_type_errors") => options.collects_type_errors = true,
                with_a_value if with_a_value.path().is_ident("collect_type_errors") => {
                    return Err(Error::new_spanned(with_a_value, "`collect_type_errors` takes no value"));
                }
                Meta::NameValue(crate_path) if crate_path.path.is_ident("crate") => {
                    if options.crate_path.is_some() {
                        return Err(Error::new_spanned(
                            &crate_path.path,
                            "`crate` is given twice: `#[annotated(...)]` takes each option once",
                        ));
                    }
                    options.crate_path = Some(path_in(&crate_path)?);
                }
                crate_path if crate_path.path().is_ident("crate") => {
                    return Err(Error::new_spanned(crate_path, CRATE_PATH));
                }
                unknown => return Err(Error::new_spanned(unknown.path(), ACCEPTED)),
            }
        }
        Ok(options)
    }
}

fn path_in(crate_path: &MetaNameValue) -> syn::Result<Path> {
    let Expr::Lit(ExprLit {
        lit: Lit::Str(path), ..
    }) = &crate_path.value
    else {
        return Err(Error::new_spanned(&crate_path.value, CRATE_PATH));
    };
    // The path's tokens take the string's span, so an error resolving the path points at the string.
    path.parse_with(Path::parse_mod_style)
        .map_err(|_| Error::new(path.span(), CRATE_PATH))
}

fn refuse_options_inside(input: &DeriveInput) -> syn::Result<()> {
    let container = if matches!(input.data, Data::Enum(_)) {
        "enum"
    } else {
        "struct"
    };
    match &input.data {
        Data::Struct(data) => {
            for field in &data.fields {
                refuse_options_on("a field", container, &field.attrs)?;
            }
        }
        Data::Enum(data) => {
            for variant in &data.variants {
                refuse_options_on("a variant", container, &variant.attrs)?;
                for field in &variant.fields {
                    refuse_options_on("a field", container, &field.attrs)?;
                }
            }
        }
        Data::Union(_) => {}
    }
    Ok(())
}

fn refuse_options_on(item: &str, container: &str, attributes: &[Attribute]) -> syn::Result<()> {
    let Some(option) = options_in(attributes)?.into_iter().next() else {
        return Ok(());
    };
    Err(Error::new_spanned(
        option.path(),
        format!("`#[annotated(...)]` takes no options on {item}: they go on the {container}"),
    ))
}

fn options_in(attributes: &[Attribute]) -> syn::Result<Vec<Meta>> {
    let mut options = Vec::new();
    for attribute in attributes
        .iter()
        .filter(|attribute| attribute.path().is_ident("annotated"))
    {
        options.extend(attribute.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?);
    }
    Ok(options)
}
