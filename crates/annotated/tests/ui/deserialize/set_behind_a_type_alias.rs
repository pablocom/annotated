use std::collections::{BTreeSet, HashSet};

use annotated::Deserialize;

type Tags = HashSet<String>;
type Sections = BTreeSet<String>;

#[derive(Deserialize)]
struct Article {
    #[validate(each(length(max = 10)))]
    tags: Tags,
    #[validate(each(length(max = 10)))]
    sections: Option<Sections>,
}

fn main() {}
