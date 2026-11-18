use std::collections::{BTreeSet, HashMap, HashSet};

use annotated::Deserialize;

#[derive(Deserialize)]
struct Shelves {
    #[validate(each(each(length(max = 10))))]
    shelves: Vec<HashSet<String>>,
}

#[derive(Deserialize)]
struct Catalogue {
    #[validate(each(each(length(max = 10))))]
    sections_by_language: HashMap<String, BTreeSet<String>>,
}

#[derive(Deserialize)]
struct Archive {
    #[validate(each(length(max = 10)))]
    tags: Box<HashSet<String>>,
}

#[derive(Deserialize)]
struct Draft {
    #[validate(each(length(max = 10)))]
    tags: Option<Box<BTreeSet<String>>>,
}

#[derive(Deserialize)]
struct Shortlists {
    #[validate(each(length(max = 3)))]
    shortlists: Vec<HashSet<String>>,
}

fn main() {}
