use std::collections::{BTreeMap, HashMap, VecDeque};
use std::fmt::{self, Display};
use std::hash::{BuildHasher, Hash};
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::Arc;

use serde::Deserializer;
use serde::de::{Error as _, MapAccess, SeqAccess, Visitor};

use crate::deserialize::DeserializeAt;
use crate::{Deserialize, Location, Report};

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Option<T> {
    fn deserialize_at<D: Deserializer<'de>>(
        deserializer: D,
        at: &Location<'_>,
        found: &mut Report,
    ) -> Result<Option<Self>, D::Error> {
        deserializer.deserialize_option(VisitorAt::<Self>::new(at, found))
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Box<T> {
    fn deserialize_at<D: Deserializer<'de>>(
        deserializer: D,
        at: &Location<'_>,
        found: &mut Report,
    ) -> Result<Option<Self>, D::Error> {
        let value = T::deserialize_at(deserializer, at, found)?;
        Ok(value.map(Box::new))
    }
}

// Built from the `T` read, as `Box<T>` is, so neither needs serde's `rc` feature.
macro_rules! deserializes_the_value_it_points_to {
    ($($pointer:ident),*) => {
        $(
            impl<'de, T: Deserialize<'de>> Deserialize<'de> for $pointer<T> {
                fn deserialize_at<D: Deserializer<'de>>(
                    deserializer: D,
                    at: &Location<'_>,
                    found: &mut Report,
                ) -> Result<Option<Self>, D::Error> {
                    let value = T::deserialize_at(deserializer, at, found)?;
                    Ok(value.map($pointer::new))
                }
            }
        )*
    };
}

deserializes_the_value_it_points_to!(Rc, Arc);

impl<'de, T: Deserialize<'de>, const N: usize> Deserialize<'de> for [T; N] {
    fn deserialize_at<D: Deserializer<'de>>(
        deserializer: D,
        at: &Location<'_>,
        found: &mut Report,
    ) -> Result<Option<Self>, D::Error> {
        deserializer.deserialize_tuple(N, VisitorAt::<Self>::new(at, found))
    }
}

macro_rules! deserializes_each_element_at_its_index {
    ($($size:literal => ($($index:tt $element:ident $read:ident),+))+) => {
        $(
            #[diagnostic::do_not_recommend]
            impl<'de, $($element: Deserialize<'de>),+> Deserialize<'de> for ($($element,)+) {
                fn deserialize_at<D: Deserializer<'de>>(
                    deserializer: D,
                    at: &Location<'_>,
                    found: &mut Report,
                ) -> Result<Option<Self>, D::Error> {
                    deserializer.deserialize_tuple($size, VisitorAt::<Self>::new(at, found))
                }
            }

            impl<'de, $($element: Deserialize<'de>),+> Visitor<'de> for VisitorAt<'_, '_, ($($element,)+)> {
                type Value = Option<($($element,)+)>;

                fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                    formatter.write_str(concat!("a tuple of size ", $size))
                }

                fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                    $(
                        let element_at = self.at.index($index);
                        let Some($read) = seq.next_element_seed(DeserializeAt::<$element>::new(&element_at, self.found))? else {
                            return Err(A::Error::invalid_length($index, &self));
                        };
                    )+
                    match ($($read,)+) {
                        ($(Some($read),)+) => Ok(Some(($($read,)+))),
                        _ => Ok(None),
                    }
                }
            }
        )+
    };
}

// serde's own tuples go up to 16 elements.
deserializes_each_element_at_its_index! {
    1 => (0 T0 read0)
    2 => (0 T0 read0, 1 T1 read1)
    3 => (0 T0 read0, 1 T1 read1, 2 T2 read2)
    4 => (0 T0 read0, 1 T1 read1, 2 T2 read2, 3 T3 read3)
    5 => (0 T0 read0, 1 T1 read1, 2 T2 read2, 3 T3 read3, 4 T4 read4)
    6 => (0 T0 read0, 1 T1 read1, 2 T2 read2, 3 T3 read3, 4 T4 read4, 5 T5 read5)
    7 => (0 T0 read0, 1 T1 read1, 2 T2 read2, 3 T3 read3, 4 T4 read4, 5 T5 read5, 6 T6 read6)
    8 => (0 T0 read0, 1 T1 read1, 2 T2 read2, 3 T3 read3, 4 T4 read4, 5 T5 read5, 6 T6 read6, 7 T7 read7)
    9 => (
        0 T0 read0, 1 T1 read1, 2 T2 read2, 3 T3 read3, 4 T4 read4, 5 T5 read5, 6 T6 read6, 7 T7 read7, 8 T8 read8
    )
    10 => (
        0 T0 read0, 1 T1 read1, 2 T2 read2, 3 T3 read3, 4 T4 read4, 5 T5 read5, 6 T6 read6, 7 T7 read7, 8 T8 read8,
        9 T9 read9
    )
    11 => (
        0 T0 read0, 1 T1 read1, 2 T2 read2, 3 T3 read3, 4 T4 read4, 5 T5 read5, 6 T6 read6, 7 T7 read7, 8 T8 read8,
        9 T9 read9, 10 T10 read10
    )
    12 => (
        0 T0 read0, 1 T1 read1, 2 T2 read2, 3 T3 read3, 4 T4 read4, 5 T5 read5, 6 T6 read6, 7 T7 read7, 8 T8 read8,
        9 T9 read9, 10 T10 read10, 11 T11 read11
    )
    13 => (
        0 T0 read0, 1 T1 read1, 2 T2 read2, 3 T3 read3, 4 T4 read4, 5 T5 read5, 6 T6 read6, 7 T7 read7, 8 T8 read8,
        9 T9 read9, 10 T10 read10, 11 T11 read11, 12 T12 read12
    )
    14 => (
        0 T0 read0, 1 T1 read1, 2 T2 read2, 3 T3 read3, 4 T4 read4, 5 T5 read5, 6 T6 read6, 7 T7 read7, 8 T8 read8,
        9 T9 read9, 10 T10 read10, 11 T11 read11, 12 T12 read12, 13 T13 read13
    )
    15 => (
        0 T0 read0, 1 T1 read1, 2 T2 read2, 3 T3 read3, 4 T4 read4, 5 T5 read5, 6 T6 read6, 7 T7 read7, 8 T8 read8,
        9 T9 read9, 10 T10 read10, 11 T11 read11, 12 T12 read12, 13 T13 read13, 14 T14 read14
    )
    16 => (
        0 T0 read0, 1 T1 read1, 2 T2 read2, 3 T3 read3, 4 T4 read4, 5 T5 read5, 6 T6 read6, 7 T7 read7, 8 T8 read8,
        9 T9 read9, 10 T10 read10, 11 T11 read11, 12 T12 read12, 13 T13 read13, 14 T14 read14, 15 T15 read15
    )
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Box<[T]> {
    fn deserialize_at<D: Deserializer<'de>>(
        deserializer: D,
        at: &Location<'_>,
        found: &mut Report,
    ) -> Result<Option<Self>, D::Error> {
        let elements = Vec::<T>::deserialize_at(deserializer, at, found)?;
        Ok(elements.map(Vec::into_boxed_slice))
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Vec<T> {
    fn deserialize_at<D: Deserializer<'de>>(
        deserializer: D,
        at: &Location<'_>,
        found: &mut Report,
    ) -> Result<Option<Self>, D::Error> {
        deserializer.deserialize_seq(VisitorAt::<Self>::new(at, found))
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for VecDeque<T> {
    fn deserialize_at<D: Deserializer<'de>>(
        deserializer: D,
        at: &Location<'_>,
        found: &mut Report,
    ) -> Result<Option<Self>, D::Error> {
        deserializer.deserialize_seq(VisitorAt::<Self>::new(at, found))
    }
}

impl<'de, K, V> Deserialize<'de> for BTreeMap<K, V>
where
    K: serde::Deserialize<'de> + Display + Ord,
    V: Deserialize<'de>,
{
    fn deserialize_at<D: Deserializer<'de>>(
        deserializer: D,
        at: &Location<'_>,
        found: &mut Report,
    ) -> Result<Option<Self>, D::Error> {
        deserializer.deserialize_map(VisitorAt::<Self>::new(at, found))
    }
}

/// A key is cloned when its value was built but has issues, so that they can be reported in the order the map
/// iterates over its values.
impl<'de, K, V, S> Deserialize<'de> for HashMap<K, V, S>
where
    K: serde::Deserialize<'de> + Display + Eq + Hash + Clone,
    V: Deserialize<'de>,
    S: BuildHasher + Default,
{
    fn deserialize_at<D: Deserializer<'de>>(
        deserializer: D,
        at: &Location<'_>,
        found: &mut Report,
    ) -> Result<Option<Self>, D::Error> {
        deserializer.deserialize_map(VisitorAt::<Self>::new(at, found))
    }
}

struct VisitorAt<'a, 'l, C> {
    at: &'a Location<'l>,
    found: &'a mut Report,
    container: PhantomData<fn() -> C>,
}

impl<'a, 'l, C> VisitorAt<'a, 'l, C> {
    fn new(at: &'a Location<'l>, found: &'a mut Report) -> Self {
        Self {
            at,
            found,
            container: PhantomData,
        }
    }

    fn read_elements<'de, A, T>(
        self,
        mut seq: A,
        mut elements: C,
        mut push: impl FnMut(&mut C, T),
    ) -> Result<Option<C>, A::Error>
    where
        A: SeqAccess<'de>,
        T: Deserialize<'de>,
    {
        let mut index = 0;
        let mut is_every_element_built = true;
        while let Some(element) = seq.next_element_seed(DeserializeAt::<T>::new(&self.at.index(index), self.found))? {
            match element {
                Some(element) => push(&mut elements, element),
                None => is_every_element_built = false,
            }
            index += 1;
        }
        Ok(is_every_element_built.then_some(elements))
    }
}

impl<'de, T: Deserialize<'de>> Visitor<'de> for VisitorAt<'_, '_, Option<T>> {
    type Value = Option<Option<T>>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("option")
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(Some(None))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(Some(None))
    }

    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        let value = T::deserialize_at(deserializer, self.at, self.found)?;
        Ok(value.map(Some))
    }
}

impl<'de, T: Deserialize<'de>> Visitor<'de> for VisitorAt<'_, '_, Vec<T>> {
    type Value = Option<Vec<T>>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a sequence")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, seq: A) -> Result<Self::Value, A::Error> {
        let elements = Vec::with_capacity(room_for::<T>(seq.size_hint()));
        self.read_elements(seq, elements, Vec::push)
    }
}

impl<'de, T: Deserialize<'de>> Visitor<'de> for VisitorAt<'_, '_, VecDeque<T>> {
    type Value = Option<VecDeque<T>>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a sequence")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, seq: A) -> Result<Self::Value, A::Error> {
        let elements = VecDeque::with_capacity(room_for::<T>(seq.size_hint()));
        self.read_elements(seq, elements, VecDeque::push_back)
    }
}

impl<'de, T: Deserialize<'de>, const N: usize> Visitor<'de> for VisitorAt<'_, '_, [T; N]> {
    type Value = Option<[T; N]>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if N == 0 {
            return formatter.write_str("an empty array");
        }
        write!(formatter, "an array of length {N}")
    }

    // Read in place, as serde's arrays are, so that reading one allocates nothing of its own.
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut elements: [Option<T>; N] = [const { None }; N];
        let mut is_every_element_built = true;
        for (index, element) in elements.iter_mut().enumerate() {
            let element_at = self.at.index(index);
            let Some(read) = seq.next_element_seed(DeserializeAt::<T>::new(&element_at, self.found))? else {
                return Err(A::Error::invalid_length(index, &self));
            };
            is_every_element_built &= read.is_some();
            *element = read;
        }
        Ok(is_every_element_built.then(|| elements.map(|element| element.expect("every element was built"))))
    }
}

impl<'de, K, V> Visitor<'de> for VisitorAt<'_, '_, BTreeMap<K, V>>
where
    K: serde::Deserialize<'de> + Display + Ord,
    V: Deserialize<'de>,
{
    type Value = Option<BTreeMap<K, V>>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a map")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut values = BTreeMap::new();
        let mut values_with_issues = BTreeMap::new();
        let mut held = HeldIssues::reported_into(self.found);
        while let Some(key) = map.next_key::<K>()? {
            let mut found_in_value = held.report_for_the_next_value();
            let value = map.next_value_seed(DeserializeAt::<V>::new(&self.at.key(&key), &mut found_in_value))?;
            match value {
                Some(value) if found_in_value.is_empty() => {
                    if let Some((_, replaced)) = values_with_issues.remove(&key) {
                        held.release(&replaced);
                    }
                    values.insert(key, value);
                }
                value => {
                    values.remove(&key);
                    held.hold(&found_in_value);
                    if let Some((_, replaced)) = values_with_issues.insert(key, (value, found_in_value)) {
                        held.release(&replaced);
                    }
                }
            }
        }
        let mut is_every_value_built = true;
        let mut joined = JoinedIssues::into(self.found);
        for (key, (value, mut found_in_value)) in values_with_issues {
            joined.join(&mut found_in_value);
            match value {
                Some(value) => {
                    values.insert(key, value);
                }
                None => is_every_value_built = false,
            }
        }
        joined.end();
        Ok(is_every_value_built.then_some(values))
    }
}

impl<'de, K, V, S> Visitor<'de> for VisitorAt<'_, '_, HashMap<K, V, S>>
where
    K: serde::Deserialize<'de> + Display + Eq + Hash + Clone,
    V: Deserialize<'de>,
    S: BuildHasher + Default,
{
    type Value = Option<HashMap<K, V, S>>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a map")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut values = HashMap::with_capacity_and_hasher(room_for::<(K, V)>(map.size_hint()), S::default());
        let mut issues_by_key = IssuesByKey::<K, S>::reported_into(self.found);
        while let Some(key) = map.next_key::<K>()? {
            let mut found_in_value = issues_by_key.report_for_the_next_value();
            let value = map.next_value_seed(DeserializeAt::<V>::new(&self.at.key(&key), &mut found_in_value))?;
            match value {
                Some(value) if found_in_value.is_empty() => {
                    issues_by_key.forget(&key);
                    values.insert(key, value);
                }
                Some(value) => {
                    issues_by_key.keep(key.clone(), found_in_value);
                    values.insert(key, value);
                }
                None => {
                    values.remove(&key);
                    issues_by_key.keep(key, found_in_value);
                }
            }
        }
        let is_every_value_built = issues_by_key.has_only_keys_of(&values);
        issues_by_key.join_in_the_order_of(&values, self.found);
        Ok(is_every_value_built.then_some(values))
    }
}

// A map reports its values' issues in its own order, and the value it reads last may be the one it reports first.
// So it holds each value's issues apart until it has read every entry. A value keeps its issues until those held
// reach what the map's report has room for, and only its first after that. The map then holds at most that room
// plus one issue for each value read later, and every value keeps its first issue, so failing fast stays exact.
// A map whose report is already finished keeps none.
struct HeldIssues {
    room: usize,
    held: usize,
    most_issues_of_the_next_value: usize,
}

impl HeldIssues {
    fn reported_into(found: &Report) -> Self {
        let room = found.room();
        Self {
            room,
            held: 0,
            most_issues_of_the_next_value: room,
        }
    }

    fn report_for_the_next_value(&self) -> Report {
        Report::keeping_at_most(self.most_issues_of_the_next_value)
    }

    fn hold(&mut self, found_in_value: &Report) {
        self.held += found_in_value.len();
        self.make_room_for_the_next_value();
    }

    fn release(&mut self, found_in_value: &Report) {
        self.held -= found_in_value.len();
        self.make_room_for_the_next_value();
    }

    fn make_room_for_the_next_value(&mut self) {
        let room_left = self.room.saturating_sub(self.held);
        let least_issues_kept = self.room.min(1);
        self.most_issues_of_the_next_value = room_left.max(least_issues_kept);
    }
}

struct IssuesByKey<K, S> {
    issues_by_key: Option<HashMap<K, Report, S>>,
    held: HeldIssues,
}

impl<K: Eq + Hash, S: BuildHasher + Default> IssuesByKey<K, S> {
    fn reported_into(found: &Report) -> Self {
        Self {
            issues_by_key: None,
            held: HeldIssues::reported_into(found),
        }
    }

    fn report_for_the_next_value(&self) -> Report {
        self.held.report_for_the_next_value()
    }

    fn keep(&mut self, key: K, found_in_value: Report) {
        self.held.hold(&found_in_value);
        let replaced = self
            .issues_by_key
            .get_or_insert_with(|| HashMap::with_hasher(S::default()))
            .insert(key, found_in_value);
        if let Some(replaced) = replaced {
            self.held.release(&replaced);
        }
    }

    fn forget(&mut self, key: &K) {
        if let Some(issues_by_key) = &mut self.issues_by_key
            && let Some(forgotten) = issues_by_key.remove(key)
        {
            self.held.release(&forgotten);
        }
    }

    fn has_only_keys_of<V>(&self, values: &HashMap<K, V, S>) -> bool {
        self.issues_by_key
            .iter()
            .flat_map(HashMap::keys)
            .all(|key| values.contains_key(key))
    }

    fn join_in_the_order_of<V>(self, values: &HashMap<K, V, S>, found: &mut Report) {
        let Some(mut issues_by_key) = self.issues_by_key else {
            return;
        };
        let mut joined = JoinedIssues::into(found);
        for key in values.keys() {
            if let Some(mut found_in_value) = issues_by_key.remove(key) {
                joined.join(&mut found_in_value);
            }
        }
        for mut found_in_value in issues_by_key.into_values() {
            joined.join(&mut found_in_value);
        }
        joined.end();
    }
}

// Joins the issues a map held for its values into its report, in the map's order. A value whose report is finished
// may have more issues than it kept, but the values after it join theirs all the same, to fill the report up to its
// limit. Once they have, a report that still has room keeps no more, since issues may be missing among those it
// holds.
struct JoinedIssues<'a> {
    found: &'a mut Report,
    may_some_be_missing: bool,
}

impl<'a> JoinedIssues<'a> {
    const fn into(found: &'a mut Report) -> Self {
        Self {
            found,
            may_some_be_missing: false,
        }
    }

    fn join(&mut self, found_in_value: &mut Report) {
        self.may_some_be_missing |= found_in_value.is_finished();
        self.found.take_issues_of(found_in_value);
    }

    fn end(self) {
        if self.may_some_be_missing {
            self.found.keep_no_more();
        }
    }
}

// serde's own cap on the room a size hint reserves: the hint is the input's word, so trusting it whole would let
// a short input ask for any amount of memory.
pub(crate) fn room_for<Element>(size_hint: Option<usize>) -> usize {
    const MOST_BYTES_RESERVED: usize = 1024 * 1024;
    match size_of::<Element>() {
        0 => 0,
        element_size => size_hint.unwrap_or(0).min(MOST_BYTES_RESERVED / element_size),
    }
}
