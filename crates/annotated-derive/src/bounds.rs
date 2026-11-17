use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::punctuated::Pair;
use syn::{
    GenericArgument, Generics, Ident, Path, PathArguments, ReturnType, Type, TypeParamBound, TypePath, WherePredicate,
    parse_quote,
};

use crate::validated_struct::ungrouped;

// The bounds of an impl, decided as serde's derive decides them: each type parameter that a value read or checked
// uses is bounded by the trait it is read or checked through, and so is an associated type of a parameter used as a
// value's whole type. A parameter used only in `PhantomData` needs no bound.
//
// Through a trait the derive implements itself, a parameter used only in the type deriving needs no bound either: that
// impl is the one being written, and has every bound it needs. So a recursive `children: Vec<Tree<T>>` asks of `T`
// what `Tree<T>` does, where bounding `T` by the trait would ask more of it than the type does.
pub struct Bounds<'a> {
    type_parameters: Vec<&'a Ident>,
    implemented_for: Option<&'a Ident>,
    predicates: Vec<WherePredicate>,
}

// A value's type, and the part of it read or checked through another trait, when there is one.
pub struct Used<'a> {
    pub value_type: &'a Type,
    pub apart: Option<&'a Type>,
}

impl<'a> Bounds<'a> {
    // Bounds by traits the derive implements for the type `deriving` too.
    pub fn implemented_here(generics: &'a Generics, deriving: &'a Ident) -> Self {
        Self {
            implemented_for: Some(deriving),
            ..Self::implemented_elsewhere(generics)
        }
    }

    pub fn implemented_elsewhere(generics: &'a Generics) -> Self {
        Self {
            type_parameters: generics.type_params().map(|parameter| &parameter.ident).collect(),
            implemented_for: None,
            predicates: Vec::new(),
        }
    }

    // Bounds every parameter `used` uses by `bound`, but those in the part apart.
    pub fn add(&mut self, used: &Used, bound: &TokenStream) {
        if self.type_parameters.is_empty() {
            return;
        }
        let mut finder = ParameterFinder {
            type_parameters: &self.type_parameters,
            implemented_for: self.implemented_for,
            apart: used.apart,
            parameters: Vec::new(),
            associated_types: Vec::new(),
        };
        finder.visit_value_type(used.value_type);
        let ParameterFinder {
            parameters,
            associated_types,
            ..
        } = finder;
        let bounded = self
            .type_parameters
            .iter()
            .filter(|parameter| parameters.contains(parameter))
            .map(|parameter| parse_quote!(#parameter: #bound))
            .chain(
                associated_types
                    .into_iter()
                    .map(|associated_type| parse_quote!(#associated_type: #bound)),
            );
        for predicate in bounded {
            add_once(&mut self.predicates, predicate);
        }
    }

    pub fn into_predicates(self) -> Vec<WherePredicate> {
        self.predicates
    }
}

pub fn add_once(predicates: &mut Vec<WherePredicate>, predicate: WherePredicate) {
    let tokens = predicate.to_token_stream().to_string();
    if !predicates
        .iter()
        .any(|added| added.to_token_stream().to_string() == tokens)
    {
        predicates.push(predicate);
    }
}

impl<'a> Used<'a> {
    pub const fn whole(value_type: &'a Type) -> Self {
        Self {
            value_type,
            apart: None,
        }
    }
}

struct ParameterFinder<'a, 'b> {
    type_parameters: &'b [&'a Ident],
    implemented_for: Option<&'b Ident>,
    apart: Option<&'b Type>,
    parameters: Vec<&'a Ident>,
    associated_types: Vec<TypePath>,
}

impl ParameterFinder<'_, '_> {
    fn visit_value_type(&mut self, value_type: &Type) {
        if self.is_apart(value_type) {
            return;
        }
        if let Type::Path(type_path) = ungrouped(value_type)
            && let Some(Pair::Punctuated(first, _)) = type_path.path.segments.pairs().next()
            && self.type_parameters.iter().any(|parameter| first.ident == **parameter)
        {
            self.associated_types.push(type_path.clone());
        }
        self.visit_type(value_type);
    }

    fn is_apart(&self, written_type: &Type) -> bool {
        self.apart.is_some_and(|apart| std::ptr::eq(apart, written_type))
    }

    fn visit_type(&mut self, written_type: &Type) {
        if self.is_apart(written_type) {
            return;
        }
        match written_type {
            Type::Array(array) => self.visit_type(&array.elem),
            Type::FnPtr(function) => {
                for input in &function.inputs {
                    self.visit_type(&input.ty);
                }
                self.visit_return_type(&function.output);
            }
            Type::Group(group) => self.visit_type(&group.elem),
            Type::Paren(parenthesized) => self.visit_type(&parenthesized.elem),
            Type::ImplTrait(impl_trait) => self.visit_bounds(impl_trait.bounds.iter()),
            Type::TraitObject(trait_object) => self.visit_bounds(trait_object.bounds.iter()),
            Type::Path(type_path) => {
                if let Some(qualified_self) = &type_path.qself {
                    self.visit_type(&qualified_self.ty);
                }
                self.visit_path(&type_path.path);
            }
            Type::Ptr(pointer) => self.visit_type(&pointer.elem),
            Type::Reference(reference) => self.visit_type(&reference.elem),
            Type::Slice(slice) => self.visit_type(&slice.elem),
            Type::Tuple(tuple) => {
                for element in &tuple.elems {
                    self.visit_type(element);
                }
            }
            _ => {}
        }
    }

    fn visit_path(&mut self, path: &Path) {
        let Some(last_segment) = path.segments.last() else {
            return;
        };
        // `PhantomData<T>` deserializes and validates whatever `T` is.
        let is_implemented_here = self
            .implemented_for
            .is_some_and(|deriving| last_segment.ident == *deriving);
        if last_segment.ident == "PhantomData" || is_implemented_here {
            return;
        }
        if path.leading_colon.is_none()
            && path.segments.len() == 1
            && let Some(parameter) = self
                .type_parameters
                .iter()
                .find(|parameter| last_segment.ident == ***parameter)
            && !self.parameters.contains(parameter)
        {
            self.parameters.push(parameter);
        }
        for segment in &path.segments {
            self.visit_path_arguments(&segment.arguments);
        }
    }

    fn visit_path_arguments(&mut self, arguments: &PathArguments) {
        match arguments {
            PathArguments::None => {}
            PathArguments::AngleBracketed(arguments) => {
                for argument in &arguments.args {
                    match argument {
                        GenericArgument::Type(argument) => self.visit_type(argument),
                        GenericArgument::AssocType(associated) => self.visit_type(&associated.ty),
                        _ => {}
                    }
                }
            }
            PathArguments::Parenthesized(arguments) => {
                for input in &arguments.inputs {
                    self.visit_type(&input.ty);
                }
                self.visit_return_type(&arguments.output);
            }
        }
    }

    fn visit_return_type(&mut self, return_type: &ReturnType) {
        if let ReturnType::Type(_, output) = return_type {
            self.visit_type(output);
        }
    }

    fn visit_bounds<'b>(&mut self, bounds: impl Iterator<Item = &'b TypeParamBound>) {
        for bound in bounds {
            if let TypeParamBound::Trait(trait_bound) = bound {
                self.visit_path(&trait_bound.path);
            }
        }
    }
}
