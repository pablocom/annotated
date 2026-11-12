//! The containers a nested value is held in, `Option`, `Box`, sequences and maps, read every value in them
//! collecting its violations: a sequence's element at its index, a map's value at its key.

#![cfg(feature = "serde")]

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::hash::{BuildHasherDefault, Hasher};

use annotated::{Deserialize, FoundViolations, Location, Violation};
use serde::Deserializer;
use serde::de::value::{MapDeserializer, SeqAccessDeserializer, SeqDeserializer};
use serde::de::{DeserializeSeed, IntoDeserializer, SeqAccess, Visitor};

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Address {
    #[validate(required)]
    street: String,
    #[validate(length(min = 5))]
    postal_code: String,
}

fn gran_via() -> Address {
    Address {
        street: "Gran Vía 1".to_owned(),
        postal_code: "28013".to_owned(),
    }
}

fn read<'de, T: Deserialize<'de>>(json: &'de str) -> (Option<T>, Vec<String>) {
    read_into(json, FoundViolations::new())
}

fn read_failing_fast<'de, T: Deserialize<'de>>(json: &'de str) -> (Option<T>, Vec<String>) {
    read_into(json, FoundViolations::failing_fast())
}

fn read_into<'de, T: Deserialize<'de>>(json: &'de str, mut found: FoundViolations) -> (Option<T>, Vec<String>) {
    let mut deserializer = serde_json::Deserializer::from_str(json);
    let value = T::deserialize_at(&mut deserializer, &Location::root(), &mut found).unwrap();
    let violations = found.into_result().map_or_else(
        |violations| violations.iter().map(path_and_code).collect(),
        |()| Vec::new(),
    );
    (value, violations)
}

fn path_and_code(violation: &Violation) -> String {
    let code = violation.problem().code();
    if violation.path().is_root() {
        return code.to_owned();
    }
    format!("{}: {code}", violation.path())
}

fn violations_reading<'de, T: Deserialize<'de>>(json: &'de str) -> Vec<String> {
    read::<T>(json).1
}

type ValueError = serde::de::value::Error;

#[test]
fn every_element_of_a_list_is_read_at_its_index() {
    let body = r#"[{ "street": "Gran Vía 1", "postalCode": "28013" }, { "postalCode": "280" }]"#;

    assert_eq!(
        violations_reading::<Vec<Address>>(body),
        ["[1].street: missing", "[1].postalCode: length"]
    );
}

#[test]
fn a_list_whose_elements_only_break_rules_is_built() {
    let body = r#"[{ "street": "", "postalCode": "28013" }]"#;

    let (list, _) = read::<Vec<Address>>(body);

    assert_eq!(
        list,
        Some(vec![Address {
            street: String::new(),
            ..gran_via()
        }])
    );
}

#[test]
fn a_list_with_an_element_that_could_not_be_built_is_not_built() {
    let body = r#"[{ "street": "Gran Vía 1", "postalCode": "28013" }, { "postalCode": "28013" }]"#;

    assert_eq!(
        read::<Vec<Address>>(body),
        (None, vec!["[1].street: missing".to_owned()])
    );
}

#[test]
fn the_elements_after_one_that_could_not_be_built_are_read() {
    let body = r#"[{ "postalCode": "28013" }, { "street": "", "postalCode": "28013" }]"#;

    assert_eq!(
        violations_reading::<Vec<Address>>(body),
        ["[0].street: missing", "[1].street: required"]
    );
}

#[test]
fn every_element_of_a_queue_is_read_at_its_index() {
    let body = r#"[{ "street": "Gran Vía 1", "postalCode": "28013" }, { "street": "", "postalCode": "28013" }]"#;

    assert_eq!(violations_reading::<VecDeque<Address>>(body), ["[1].street: required"]);
}

#[test]
fn every_element_of_a_boxed_slice_is_read_at_its_index() {
    let body = r#"[{ "street": "Gran Vía 1", "postalCode": "28013" }, { "street": "", "postalCode": "28013" }]"#;

    assert_eq!(violations_reading::<Box<[Address]>>(body), ["[1].street: required"]);
}

#[test]
fn a_boxed_slice_is_built_from_its_elements() {
    let body = r#"[{ "street": "Gran Vía 1", "postalCode": "28013" }]"#;

    let (boxed_slice, _) = read::<Box<[Address]>>(body);

    assert_eq!(boxed_slice, Some(vec![gran_via()].into_boxed_slice()));
}

#[test]
fn a_boxed_value_is_read_with_its_violations() {
    let body = r#"{ "street": "", "postalCode": "28013" }"#;

    assert_eq!(
        read::<Box<Address>>(body),
        (
            Some(Box::new(Address {
                street: String::new(),
                ..gran_via()
            })),
            vec!["street: required".to_owned()]
        )
    );
}

#[test]
fn null_is_read_as_no_value() {
    assert_eq!(read::<Option<Address>>("null"), (Some(None), Vec::new()));
}

struct EveryValueAsUnit;

impl<'de> Deserializer<'de> for EveryValueAsUnit {
    type Error = ValueError;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, ValueError> {
        visitor.visit_unit()
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes byte_buf option unit unit_struct
        newtype_struct seq tuple tuple_struct map struct enum identifier ignored_any
    }
}

#[test]
fn unit_is_read_as_no_value() {
    let read = Option::<Address>::deserialize_at(EveryValueAsUnit, &Location::root(), &mut FoundViolations::new());

    assert_eq!(read, Ok(Some(None)));
}

#[test]
fn a_value_in_an_option_is_read_with_its_violations() {
    let body = r#"{ "street": "", "postalCode": "28013" }"#;

    assert_eq!(
        read::<Option<Address>>(body),
        (
            Some(Some(Address {
                street: String::new(),
                ..gran_via()
            })),
            vec!["street: required".to_owned()]
        )
    );
}

#[test]
fn an_option_whose_value_could_not_be_built_is_not_built() {
    let body = r#"{ "postalCode": "28013" }"#;

    assert_eq!(
        read::<Option<Address>>(body),
        (None, vec!["street: missing".to_owned()])
    );
}

#[test]
fn every_value_of_a_map_is_read_at_its_key() {
    let body = r#"{ "home": { "street": "Gran Vía 1", "postalCode": "28013" }, "work": { "postalCode": "280" } }"#;

    assert_eq!(
        violations_reading::<BTreeMap<String, Address>>(body),
        ["[work].street: missing", "[work].postalCode: length"]
    );
}

#[test]
fn the_values_of_a_map_are_reported_in_the_order_of_their_keys() {
    let body =
        r#"{ "work": { "street": "", "postalCode": "28013" }, "home": { "street": "", "postalCode": "28013" } }"#;

    assert_eq!(
        violations_reading::<BTreeMap<String, Address>>(body),
        ["[home].street: required", "[work].street: required"]
    );
}

#[test]
fn a_map_is_built_from_its_values() {
    let body = r#"{ "home": { "street": "Gran Vía 1", "postalCode": "28013" } }"#;

    let (map, _) = read::<BTreeMap<String, Address>>(body);

    assert_eq!(map, Some(BTreeMap::from([("home".to_owned(), gran_via())])));
}

#[test]
fn a_map_with_a_value_that_could_not_be_built_is_not_built() {
    let body = r#"{ "home": { "street": "Gran Vía 1", "postalCode": "28013" }, "work": { "postalCode": "28013" } }"#;

    assert_eq!(
        read::<BTreeMap<String, Address>>(body),
        (None, vec!["[work].street: missing".to_owned()])
    );
}

#[test]
fn a_value_given_again_for_a_key_replaces_the_one_before_with_its_violations() {
    let body = r#"{
        "home": { "street": "", "postalCode": "28013" },
        "home": { "street": "Gran Vía 1", "postalCode": "280" }
    }"#;

    assert_eq!(
        violations_reading::<BTreeMap<String, Address>>(body),
        ["[home].postalCode: length"]
    );
}

#[test]
fn a_valid_value_given_again_for_a_key_leaves_no_violation_of_the_one_before() {
    let body = r#"{
        "home": { "street": "", "postalCode": "28013" },
        "home": { "street": "Gran Vía 1", "postalCode": "28013" }
    }"#;

    assert_eq!(
        read::<BTreeMap<String, Address>>(body),
        (Some(BTreeMap::from([("home".to_owned(), gran_via())])), Vec::new())
    );
}

#[test]
fn a_value_with_violations_given_again_for_a_key_replaces_the_valid_one_before() {
    let body = r#"{
        "home": { "street": "Gran Vía 1", "postalCode": "28013" },
        "home": { "street": "Gran Vía 1", "postalCode": "280" }
    }"#;

    let short_postal_code = Address {
        street: "Gran Vía 1".to_owned(),
        postal_code: "280".to_owned(),
    };
    assert_eq!(
        read::<BTreeMap<String, Address>>(body),
        (
            Some(BTreeMap::from([("home".to_owned(), short_postal_code)])),
            vec!["[home].postalCode: length".to_owned()]
        )
    );
}

#[derive(Default)]
struct KeyAsItsOwnHash(u64);

impl Hasher for KeyAsItsOwnHash {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, _: &[u8]) {
        unreachable!("only u64 keys are hashed")
    }

    fn write_u64(&mut self, key: u64) {
        self.0 = key;
    }
}

type StreetNumbers = HashMap<u64, Address, BuildHasherDefault<KeyAsItsOwnHash>>;

#[test]
fn every_value_of_a_hash_map_with_its_own_hasher_is_read_at_its_key() {
    let body = r#"{ "1": { "street": "Gran Vía 1", "postalCode": "28013" }, "3": { "postalCode": "28013" } }"#;

    assert_eq!(
        read::<StreetNumbers>(body),
        (None, vec!["[3].street: missing".to_owned()])
    );
}

#[test]
fn the_values_of_a_hash_map_are_reported_in_the_order_the_map_goes_through_them() {
    let body = r#"{ "5": { "street": "", "postalCode": "28013" }, "4": { "street": "4", "postalCode": "28013" },
        "3": { "street": "3", "postalCode": "28013" }, "2": { "street": "", "postalCode": "28013" },
        "1": { "street": "1", "postalCode": "28013" } }"#;

    assert_eq!(
        violations_reading::<StreetNumbers>(body),
        ["[2].street: required", "[5].street: required"]
    );
}

#[test]
fn a_value_given_again_for_a_key_of_a_hash_map_replaces_the_one_before_with_its_violations() {
    let body =
        r#"{ "1": { "street": "", "postalCode": "28013" }, "1": { "street": "Gran Vía 1", "postalCode": "280" } }"#;

    assert_eq!(violations_reading::<StreetNumbers>(body), ["[1].postalCode: length"]);
}

#[test]
fn a_valid_value_given_again_for_a_key_of_a_hash_map_leaves_no_violation_of_the_one_before() {
    let body =
        r#"{ "1": { "street": "", "postalCode": "28013" }, "1": { "street": "Gran Vía 1", "postalCode": "28013" } }"#;

    assert_eq!(
        read::<StreetNumbers>(body),
        (Some(StreetNumbers::from_iter([(1, gran_via())])), Vec::new())
    );
}

#[test]
fn a_value_that_could_not_be_built_given_again_for_a_key_of_a_hash_map_leaves_the_map_unbuilt() {
    let body = r#"{ "1": { "street": "Gran Vía 1", "postalCode": "28013" }, "1": { "postalCode": "28013" } }"#;

    assert_eq!(
        read::<StreetNumbers>(body),
        (None, vec!["[1].street: missing".to_owned()])
    );
}

#[test]
fn a_hash_map_is_built_from_its_values() {
    let body = r#"{ "1": { "street": "Gran Vía 1", "postalCode": "28013" } }"#;

    let (map, _) = read::<StreetNumbers>(body);

    assert_eq!(map, Some(StreetNumbers::from_iter([(1, gran_via())])));
}

#[derive(Deserialize, Debug, PartialEq)]
struct Line {
    #[validate(range(min = 1))]
    quantity: u32,
}

#[test]
fn an_option_of_a_list_reads_every_element_at_its_index() {
    let body = r#"[{ "quantity": 1 }, { "quantity": 0 }]"#;

    assert_eq!(violations_reading::<Option<Vec<Line>>>(body), ["[1].quantity: range"]);
}

#[test]
fn a_map_of_lists_reads_every_element_at_its_key_and_index() {
    let body = r#"{ "monday": [{ "quantity": 1 }], "tuesday": [{ "quantity": 2 }, { "quantity": 0 }] }"#;

    assert_eq!(
        violations_reading::<BTreeMap<String, Vec<Line>>>(body),
        ["[tuesday][1].quantity: range"]
    );
}

#[test]
fn a_list_of_maps_reports_each_maps_values_in_the_order_of_their_keys_after_the_maps_before() {
    let body = r#"[{ "monday": { "quantity": 0 } }, { "wednesday": { "quantity": 0 }, "tuesday": { "quantity": 0 } }]"#;

    assert_eq!(
        violations_reading::<Vec<BTreeMap<String, Line>>>(body),
        [
            "[0][monday].quantity: range",
            "[1][tuesday].quantity: range",
            "[1][wednesday].quantity: range"
        ]
    );
}

#[test]
fn failing_fast_answers_the_first_violation_of_the_first_element_with_one() {
    let body = r#"[{ "quantity": 1 }, { "quantity": 0 }, {}]"#;

    assert_eq!(
        read_failing_fast::<Vec<Line>>(body),
        (None, vec!["[1].quantity: range".to_owned()])
    );
}

#[test]
fn failing_fast_answers_the_first_violation_of_a_map_in_the_order_of_its_keys() {
    let body = r#"{ "tuesday": { "quantity": 0 }, "monday": { "quantity": 0 } }"#;

    assert_eq!(
        read_failing_fast::<BTreeMap<String, Line>>(body).1,
        ["[monday].quantity: range"]
    );
}

#[derive(Debug, PartialEq)]
struct Quantity(u32);

impl<'de> annotated::Deserialize<'de> for Quantity {
    fn deserialize_at<D: Deserializer<'de>>(
        deserializer: D,
        _: &Location<'_>,
        _: &mut FoundViolations,
    ) -> Result<Option<Self>, D::Error> {
        Ok(Some(Self(serde::Deserialize::deserialize(deserializer)?)))
    }
}

#[test]
fn a_sequence_is_read_into_room_for_the_elements_it_says_it_has() {
    let quantities = SeqDeserializer::<_, ValueError>::new([1_u32, 2, 3, 4, 5].into_iter());

    let allocations = allocation_counter::measure(|| {
        let _ = Vec::<Quantity>::deserialize_at(quantities, &Location::root(), &mut FoundViolations::new());
    });

    assert_eq!(allocations.count_total, 1);
}

#[test]
fn a_queue_is_read_into_room_for_the_elements_it_says_it_has() {
    let quantities = SeqDeserializer::<_, ValueError>::new([1_u32, 2, 3, 4, 5].into_iter());

    let allocations = allocation_counter::measure(|| {
        let _ = VecDeque::<Quantity>::deserialize_at(quantities, &Location::root(), &mut FoundViolations::new());
    });

    assert_eq!(allocations.count_total, 1);
}

#[derive(Deserialize, Debug, PartialEq)]
struct Ping {}

#[test]
fn a_sequence_of_values_that_take_no_room_is_read() {
    assert_eq!(
        read::<Vec<Ping>>("[{}, {}]"),
        (Some(vec![Ping {}, Ping {}]), Vec::new())
    );
}

#[test]
fn a_hash_map_is_read_into_room_for_the_values_it_says_it_has() {
    let quantities = (1..=5_u64).map(|key| (key, 1_u32)).collect::<Vec<_>>();
    let quantities = MapDeserializer::<_, ValueError>::new(quantities.into_iter());

    let allocations = allocation_counter::measure(|| {
        let _ = HashMap::<u64, Quantity>::deserialize_at(quantities, &Location::root(), &mut FoundViolations::new());
    });

    assert_eq!(allocations.count_total, 1);
}

struct ClaimingMoreElements {
    elements: std::vec::IntoIter<u32>,
}

impl<'de> SeqAccess<'de> for ClaimingMoreElements {
    type Error = ValueError;

    fn next_element_seed<T: DeserializeSeed<'de>>(&mut self, seed: T) -> Result<Option<T::Value>, ValueError> {
        self.elements
            .next()
            .map(|element| seed.deserialize(element.into_deserializer()))
            .transpose()
    }

    fn size_hint(&self) -> Option<usize> {
        Some(usize::MAX)
    }
}

#[test]
fn a_sequence_saying_it_has_more_elements_than_memory_holds_is_read() {
    let quantities = ClaimingMoreElements {
        elements: vec![1, 2].into_iter(),
    };

    let read = Vec::<Quantity>::deserialize_at(
        SeqAccessDeserializer::new(quantities),
        &Location::root(),
        &mut FoundViolations::new(),
    );

    assert_eq!(read, Ok(Some(vec![Quantity(1), Quantity(2)])));
}
