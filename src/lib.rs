use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::iter::FromIterator;
use std::marker::Sized;

pub trait BoundOps: Copy + Debug + Eq + Ord
where
    Self: Sized,
{
}
impl<T> BoundOps for T where T: Copy + Debug + Eq + Ord {}

#[derive(Clone, Debug, PartialEq)]
pub struct TaggedInterval<Bound>
where
    Bound: BoundOps,
{
    lower: Bound,
    upper: Bound,
    tags: HashSet<String>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum BoundKind {
    Specified,
    History,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum BoundDirection {
    Lower,
    Upper,
}

#[derive(Clone, Debug, PartialEq)]
struct TaggedBound<Bound>
where
    Bound: BoundOps,
{
    kind: BoundKind,
    direction: BoundDirection,
    bound: Bound,
    tags: HashSet<String>,
}

impl<Bound> TaggedBound<Bound>
where
    Bound: BoundOps,
{
    fn from_interval(interval: &TaggedInterval<Bound>, kind: BoundKind) -> (Self, Self) {
        (
            Self {
                kind,
                direction: BoundDirection::Lower,
                bound: interval.lower,
                tags: interval.tags.clone(),
            },
            Self {
                kind,
                direction: BoundDirection::Upper,
                bound: interval.upper,
                tags: interval.tags.clone(),
            },
        )
    }

    fn from_history(intervals: &Vec<TaggedInterval<Bound>>) -> Vec<Self> {
        let mut bounds = vec![];
        for iv in intervals {
            let (lower, upper) = Self::from_interval(iv, BoundKind::History);
            bounds.push(lower);
            bounds.push(upper);
        }
        bounds
    }

    fn from_specified(interval: &TaggedInterval<Bound>) -> Vec<Self> {
        let mut bounds = vec![];
        let (lower, upper) = Self::from_interval(interval, BoundKind::Specified);
        bounds.push(lower);
        bounds.push(upper);
        bounds
    }

    pub fn from_intervals(
        specified: &TaggedInterval<Bound>,
        history: &Vec<TaggedInterval<Bound>>,
    ) -> Vec<Self> {
        vec![Self::from_specified(specified), Self::from_history(history)].concat()
    }

    pub fn sort(bounds: &mut Vec<Self>) -> &mut Vec<Self> {
        bounds.sort_by(|x, y| x.bound.partial_cmp(&y.bound).unwrap());
        bounds
    }
}

fn difference_with_dups(v1: &Vec<String>, v2: &Vec<String>) -> Vec<String> {
    let mut result = v1.clone();
    let mut counts: HashMap<String, i128> = HashMap::new();
    v2.iter().for_each(|s| {
        counts.entry(s.clone()).and_modify(|n| *n += 1).or_insert(1);
    });
    result.retain(|s| {
        *counts
            .entry(s.clone())
            .and_modify(|n| *n -= 1)
            .or_insert(-1)
            < 0
    });
    result
}

impl<Bound> TaggedInterval<Bound>
where
    Bound: BoundOps,
{
    pub fn new(lower: Bound, upper: Bound, tags: HashSet<String>) -> Self {
        Self { lower, upper, tags }
    }

    pub fn difference(self, history: Vec<Self>) -> Vec<Self> {
        let mut bounds = TaggedBound::from_intervals(&self, &history);
        TaggedBound::sort(&mut bounds);

        let mut result = vec![];
        let mut in_specified_range = false;
        let mut current_tags = vec![];
        let mut current_bound = self.lower;
        let num_bounds = bounds.len();
        let mut i = 0;

        while i < num_bounds {
            let mut specified_lower_found = false;
            let mut specified_range_will_be_over = false;
            let mut lower_tags = vec![];
            let mut upper_tags = vec![];
            let mut j = i;

            while j < num_bounds && bounds[j].bound.eq(&bounds[i].bound) {
                match bounds[j].kind {
                    BoundKind::History => match bounds[j].direction {
                        BoundDirection::Lower => {
                            bounds[j]
                                .tags
                                .iter()
                                .for_each(|t| lower_tags.push(t.clone()));
                        }
                        BoundDirection::Upper => {
                            bounds[j]
                                .tags
                                .iter()
                                .for_each(|t| upper_tags.push(t.clone()));
                        }
                    },
                    BoundKind::Specified => match bounds[j].direction {
                        BoundDirection::Lower => {
                            specified_lower_found = true;
                        }
                        BoundDirection::Upper => {
                            specified_range_will_be_over = true;
                        }
                    },
                }
                j += 1;
            }

            let mut next_tags = difference_with_dups(&current_tags, &upper_tags);
            next_tags.append(&mut lower_tags);

            let continuous = in_specified_range
                && HashSet::<String>::from_iter(next_tags.iter().cloned())
                    .eq(&HashSet::from_iter(current_tags.iter().cloned()));

            if in_specified_range && (!continuous || specified_range_will_be_over) {
                let current_tag_set = current_tags.iter().cloned().collect();
                let tags: HashSet<String> =
                    self.tags.difference(&current_tag_set).cloned().collect();
                if !tags.is_empty() {
                    let tagged_bound = TaggedInterval::new(current_bound, bounds[i].bound, tags);
                    result.push(tagged_bound);
                }
            }

            if specified_range_will_be_over {
                break;
            }
            if specified_lower_found {
                in_specified_range = true;
            }
            if !continuous {
                current_bound = bounds[i].bound;
            }

            i = j;
            current_tags = next_tags;
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::prelude::*;

    type Time = DateTime<Utc>;

    fn tiv<'a>(lower: Time, upper: Time, tags: HashSet<String>) -> TaggedInterval<Time> {
        TaggedInterval::new(lower, upper, tags)
    }

    fn time(s: &str) -> Time {
        s.parse::<Time>().unwrap()
    }

    fn tags(strs: &[&str]) -> HashSet<String> {
        strs.iter().cloned().map(|s| s.to_string()).collect()
    }

    #[test]
    fn difference_works() {
        let cases = vec![
            (
                "empty (zero length)",
                // specified
                tiv(
                    time("2077-07-07T09:00:00Z"),
                    time("2077-07-07T09:00:00Z"),
                    tags(&["freedom", "liberty"]),
                ),
                // history
                vec![],
                // expected
                vec![],
            ),
            (
                "empty (no tags)",
                // specified
                tiv(
                    time("2077-07-07T09:00:00Z"),
                    time("2077-07-07T17:00:00Z"),
                    tags(&[]),
                ),
                // history
                vec![tiv(
                    time("2077-07-07T09:00:00Z"),
                    time("2077-07-07T17:00:00Z"),
                    tags(&[]),
                )],
                // expected
                vec![],
            ),
            (
                "empty (exact)",
                // specified
                tiv(
                    time("2077-07-07T09:00:00Z"),
                    time("2077-07-07T17:00:00Z"),
                    tags(&["freedom", "liberty"]),
                ),
                // history
                vec![tiv(
                    time("2077-07-07T09:00:00Z"),
                    time("2077-07-07T17:00:00Z"),
                    tags(&["freedom", "liberty"]),
                )],
                // expected
                vec![],
            ),
            (
                "empty (fully covered)",
                // specified
                tiv(
                    time("2077-07-07T09:00:00Z"),
                    time("2077-07-07T17:00:00Z"),
                    tags(&["freedom", "liberty"]),
                ),
                // history
                vec![tiv(
                    time("2077-07-07T08:00:00Z"),
                    time("2077-07-07T18:00:00Z"),
                    tags(&["freedom", "liberty", "fairness"]),
                )],
                // expected
                vec![],
            ),
            (
                "empty (lower-part covered)",
                // specified
                tiv(
                    time("2077-07-07T09:00:00Z"),
                    time("2077-07-07T17:00:00Z"),
                    tags(&["freedom"]),
                ),
                // history
                vec![tiv(
                    time("2077-07-07T09:00:00Z"),
                    time("2077-07-07T18:00:00Z"),
                    tags(&["freedom"]),
                )],
                // expected
                vec![],
            ),
            (
                "empty (upper-part covered)",
                // specified
                tiv(
                    time("2077-07-07T09:00:00Z"),
                    time("2077-07-07T17:00:00Z"),
                    tags(&["freedom"]),
                ),
                // history
                vec![tiv(
                    time("2077-07-07T08:00:00Z"),
                    time("2077-07-07T17:00:00Z"),
                    tags(&["freedom"]),
                )],
                // expected
                vec![],
            ),
            (
                "through (uncovered)",
                // specified
                tiv(
                    time("2077-07-07T09:00:00Z"),
                    time("2077-07-07T17:00:00Z"),
                    tags(&["freedom", "liberty"]),
                ),
                // history
                vec![tiv(
                    time("2077-07-08T09:00:00Z"),
                    time("2077-07-08T17:00:00Z"),
                    tags(&["freedom", "liberty"]),
                )],
                // expected
                vec![tiv(
                    time("2077-07-07T09:00:00Z"),
                    time("2077-07-07T17:00:00Z"),
                    tags(&["freedom", "liberty"]),
                )],
            ),
            (
                "through (no matching tags)",
                // specified
                tiv(
                    time("2077-07-07T09:00:00Z"),
                    time("2077-07-07T17:00:00Z"),
                    tags(&["freedom", "liberty"]),
                ),
                // history
                vec![tiv(
                    time("2077-07-07T09:00:00Z"),
                    time("2077-07-07T17:00:00Z"),
                    tags(&["fairness"]),
                )],
                // expected
                vec![tiv(
                    time("2077-07-07T09:00:00Z"),
                    time("2077-07-07T17:00:00Z"),
                    tags(&["freedom", "liberty"]),
                )],
            ),
            (
                "unfetched (covered)",
                // specified
                tiv(
                    time("2077-07-07T09:00:00Z"),
                    time("2077-07-07T17:00:00Z"),
                    tags(&["freedom", "liberty", "fairness"]),
                ),
                // history
                vec![tiv(
                    time("2077-07-07T09:00:00Z"),
                    time("2077-07-07T17:00:00Z"),
                    tags(&["freedom", "liberty"]),
                )],
                // expected
                vec![tiv(
                    time("2077-07-07T09:00:00Z"),
                    time("2077-07-07T17:00:00Z"),
                    tags(&["fairness"]),
                )],
            ),
            (
                "unfetched (multiple overlappings)",
                // specified
                tiv(
                    time("2077-07-07T09:00:00Z"),
                    time("2077-07-07T17:00:00Z"),
                    tags(&["freedom", "liberty", "fairness", "democracy"]),
                ),
                // history
                vec![
                    tiv(
                        time("2077-07-07T08:00:00Z"),
                        time("2077-07-07T12:00:00Z"),
                        tags(&["freedom", "liberty"]),
                    ),
                    tiv(
                        time("2077-07-07T15:00:00Z"),
                        time("2077-07-07T18:00:00Z"),
                        tags(&["liberty", "fairness"]),
                    ),
                ],
                // expected
                vec![
                    tiv(
                        time("2077-07-07T09:00:00Z"),
                        time("2077-07-07T12:00:00Z"),
                        tags(&["fairness", "democracy"]),
                    ),
                    tiv(
                        time("2077-07-07T12:00:00Z"),
                        time("2077-07-07T15:00:00Z"),
                        tags(&["freedom", "liberty", "fairness", "democracy"]),
                    ),
                    tiv(
                        time("2077-07-07T15:00:00Z"),
                        time("2077-07-07T17:00:00Z"),
                        tags(&["freedom", "democracy"]),
                    ),
                ],
            ),
            (
                "unfetched (continuous)",
                // specified
                tiv(
                    time("2077-07-07T09:00:00Z"),
                    time("2077-07-07T17:00:00Z"),
                    tags(&["freedom", "liberty", "fairness", "democracy"]),
                ),
                // history
                vec![
                    tiv(
                        time("2077-07-07T09:00:00Z"),
                        time("2077-07-07T13:00:00Z"),
                        tags(&["freedom", "liberty"]),
                    ),
                    tiv(
                        time("2077-07-07T13:00:00Z"),
                        time("2077-07-07T17:00:00Z"),
                        tags(&["freedom", "liberty"]),
                    ),
                ],
                // expected
                vec![tiv(
                    time("2077-07-07T09:00:00Z"),
                    time("2077-07-07T17:00:00Z"),
                    tags(&["fairness", "democracy"]),
                )],
            ),
            (
                "unfetched (zigzag)",
                // specified
                tiv(
                    time("2077-07-07T09:00:00Z"),
                    time("2077-07-07T17:00:00Z"),
                    tags(&["freedom", "liberty"]),
                ),
                // history
                vec![
                    tiv(
                        time("2077-07-07T03:00:00Z"),
                        time("2077-07-07T11:00:00Z"),
                        tags(&["freedom"]),
                    ),
                    tiv(
                        time("2077-07-07T10:00:00Z"),
                        time("2077-07-07T13:00:00Z"),
                        tags(&["liberty"]),
                    ),
                    tiv(
                        time("2077-07-07T12:00:00Z"),
                        time("2077-07-07T15:00:00Z"),
                        tags(&["freedom"]),
                    ),
                    tiv(
                        time("2077-07-07T14:00:00Z"),
                        time("2077-07-07T17:00:00Z"),
                        tags(&["liberty"]),
                    ),
                    tiv(
                        time("2077-07-07T16:00:00Z"),
                        time("2077-07-07T23:59:00Z"),
                        tags(&["freedom"]),
                    ),
                ],
                // expected
                vec![
                    tiv(
                        time("2077-07-07T09:00:00Z"),
                        time("2077-07-07T10:00:00Z"),
                        tags(&["liberty"]),
                    ),
                    tiv(
                        time("2077-07-07T11:00:00Z"),
                        time("2077-07-07T12:00:00Z"),
                        tags(&["freedom"]),
                    ),
                    tiv(
                        time("2077-07-07T13:00:00Z"),
                        time("2077-07-07T14:00:00Z"),
                        tags(&["liberty"]),
                    ),
                    tiv(
                        time("2077-07-07T15:00:00Z"),
                        time("2077-07-07T16:00:00Z"),
                        tags(&["freedom"]),
                    ),
                ],
            ),
            (
                "unfetched (multiple dupes)",
                // specified
                tiv(
                    time("2077-07-07T09:00:00Z"),
                    time("2077-07-07T17:00:00Z"),
                    tags(&["freedom", "liberty", "fairness", "democracy"]),
                ),
                // history
                vec![
                    tiv(
                        time("2077-07-07T09:00:00Z"),
                        time("2077-07-07T13:00:00Z"),
                        tags(&["freedom", "liberty"]),
                    ),
                    tiv(
                        time("2077-07-07T09:00:00Z"),
                        time("2077-07-07T13:00:00Z"),
                        tags(&["liberty", "fairness"]),
                    ),
                    tiv(
                        time("2077-07-07T13:00:00Z"),
                        time("2077-07-07T15:00:00Z"),
                        tags(&["freedom", "fairness"]),
                    ),
                    tiv(
                        time("2077-07-07T13:00:00Z"),
                        time("2077-07-07T15:00:00Z"),
                        tags(&["fairness", "democracy"]),
                    ),
                    tiv(
                        time("2077-07-07T15:00:00Z"),
                        time("2077-07-07T17:00:00Z"),
                        tags(&["freedom"]),
                    ),
                    tiv(
                        time("2077-07-07T15:00:00Z"),
                        time("2077-07-07T17:00:00Z"),
                        tags(&["democracy"]),
                    ),
                ],
                // expected
                vec![
                    tiv(
                        time("2077-07-07T09:00:00Z"),
                        time("2077-07-07T13:00:00Z"),
                        tags(&["democracy"]),
                    ),
                    tiv(
                        time("2077-07-07T13:00:00Z"),
                        time("2077-07-07T15:00:00Z"),
                        tags(&["liberty"]),
                    ),
                    tiv(
                        time("2077-07-07T15:00:00Z"),
                        time("2077-07-07T17:00:00Z"),
                        tags(&["liberty", "fairness"]),
                    ),
                ],
            ),
        ];

        for (name, specified, history, expected) in cases {
            assert_eq!(specified.difference(history), expected, "{}", name)
        }
    }
}
