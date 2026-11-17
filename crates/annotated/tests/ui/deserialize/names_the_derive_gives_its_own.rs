use std::borrow::Cow;

use annotated::Deserialize;

#[derive(Deserialize)]
struct __Field {
    #[validate(required)]
    name: String,
}

#[derive(Deserialize)]
struct Page<__Visitor> {
    items: Vec<__Visitor>,
}

#[derive(Deserialize)]
struct Reading<__D> {
    value: __D,
}

#[derive(Deserialize)]
struct Note<'de> {
    #[serde(borrow)]
    #[validate(required)]
    text: Cow<'de, str>,
}

mod _serde {
    pub struct Id;
}

#[derive(Deserialize)]
struct Order {
    #[validate(custom(|_: &u8| { let _ = _serde::Id; Ok(()) }))]
    count: u8,
}

fn main() {}
