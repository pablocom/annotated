use annotated::Validate;

mod _annotated {
    pub fn is_short(_: &String) -> Result<(), annotated::Problem> {
        Ok(())
    }
}

#[derive(Validate)]
struct NewMember {
    #[validate(custom(_annotated::is_short))]
    name: String,
}

const __ANNOTATED_PATTERN_LIMIT: usize = 5;

#[derive(Validate)]
struct NewTeam {
    #[validate(length(max = __ANNOTATED_PATTERN_LIMIT))]
    name: String,
}

#[derive(Validate)]
struct __Visitor {
    #[validate(required)]
    name: String,
}

fn main() {}
