struct Address {
    street: String,
}

fn main() {
    let mut deserializer = serde_json::Deserializer::from_str(r#"{ "street": "Gran Vía 1" }"#);
    let _ = annotated::from_deserializer::<Address, _>(&mut deserializer);
}
