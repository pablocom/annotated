//! With `serde_json`'s `raw_value` feature, a `RawValue` reads the text of a value in the input. A struct collecting
//! type errors buffers its values, and a buffered value has no text, so it can't read one.

use annotated::Deserialize;
use serde_json::value::RawValue;

fn read<'de, T: Deserialize<'de>>(json: &'de str) -> Result<T, String> {
    annotated::json::from_str(json).map_err(|error| error.to_string())
}

#[derive(Deserialize, Debug)]
struct Webhook {
    payload: Box<RawValue>,
}

#[derive(Deserialize, Debug)]
#[annotated(collect_type_errors)]
struct WebhookCollectingTypeErrors {
    #[allow(dead_code)]
    payload: Box<RawValue>,
}

#[test]
fn a_raw_value_is_read_as_its_text() {
    assert_eq!(
        read::<Webhook>(r#"{ "payload": {"amount": 2.50} }"#).map(|webhook| webhook.payload.get().to_owned()),
        Ok(r#"{"amount": 2.50}"#.to_owned())
    );
}

#[test]
fn a_raw_value_is_of_the_wrong_type_under_collect_type_errors() {
    assert_eq!(
        read::<WebhookCollectingTypeErrors>(r#"{ "payload": {"amount": 2.50} }"#).map(|_| ()),
        Err("payload: must be any valid JSON value, found map".to_owned())
    );
}
