#[cfg(test)]
#[macro_use]
extern crate quickcheck;

extern crate xorshift;
extern crate arraydeque;

use arraydeque::ArrayDeque;
use std::iter::{self, FromIterator};
use std::cmp::Ordering;
use std::mem::uninitialized;


/// `StreamingMedian` provides a simple interface for inserting values
/// and calculating medians.
pub struct StreamingMedian {
    data: ArrayDeque<[u32; 64]>,
    sorted: [u32; 64],
    last_median: u32,
}

impl StreamingMedian {
    pub fn new(initial_median: u32) -> StreamingMedian {
        let data = ArrayDeque::from_iter(iter::repeat(initial_median).take(64));

        // We use unsafe here and then immediately assign values to the
        // unused space
        let mut sorted: [u32; 64] = [0; 64];

        for (i, t) in data.iter().enumerate() {
            sorted[i] = *t;
        }

        StreamingMedian {
            data,
            sorted,
            last_median: initial_median,
        }
    }

    /// Returns the last median value without performing any recalculation
    ///
    /// # Example
    /// ```norun
    /// use sqs_service_handler::autoscaling::median;
    ///
    /// let stream = StreamingMedian::new(123_000);
    /// assert_eq!(stream.last(), 31_000);
    /// ```
    pub fn last(&self) -> u32 {
        self.last_median
    }

    /// Calculates and returns the median
    ///
    /// # Arguments
    ///
    /// * `value` - The value to be inserted into the stream
    /// # Example
    /// ```norun
    /// use sqs_service_handler::autoscaling::median;
    ///
    /// let stream = StreamingMedian::new(123_000);
    /// assert_eq!(stream.insert_and_calculate(31_000), 31_000);
    /// ```
    /// The algorithm used to efficiently insert and calculate relies
    /// on the fact that the data is always left in a sorted state.
    ///
    /// First we pop off the oldest value, 'removed', from our internal
    /// ring buffer. Then we add our new value 'value' to the buffer at
    /// the back. We use this buffer to maintain a temporal relationship
    /// between our values.
    ///
    /// A separate stack array 'self.sorted' is used to maintain a sorted
    /// representation of the data.
    ///
    /// We binary search for 'removed' in our 'sorted' array and store the
    /// index as 'remove_index'.
    ///
    /// We then calculate where to insert the new 'value' by binary searching
    /// for it, either finding it already or where to insert it.
    ///
    /// If the 'insert_index' for our 'value' is less than the 'remove_index'
    /// we shift the data between the 'remove_index' and the 'insert_index' over
    /// one space. This overwrites the old value we want to remove while maintaining
    /// order. We can then insert our value into the array.
    ///
    /// Example:
    /// Starting with a self.sorted of
    /// [2, 3, 4, 5, 7, 8]
    /// We then call insert_and_calculate(6)
    /// Let's assume that '3' is the oldest value. This makes 'remove_index' = 1
    /// We search for where to insert our value '6' and its' index 3.
    /// [2, 3, 4, 5, 7, 8] <- remove_index = 1, insert_index = 3
    /// Shift the data between 1 and 3 over by one.
    /// [2, 4, 5, 5, 7, 8]
    /// Insert our value into index 3.
    /// [2, 4, 5, 6, 7, 8]
    ///
    /// A similar approach is performed in the case of the insert_index being before
    /// the remove index.
    ///
    /// Unsafe is used here to dramatically improve performance - a full 3-5x
    pub fn insert_and_calculate(&mut self, value: u32) -> u32 {
        let mut scratch_space: [u32; 64] = unsafe { uninitialized() };

        let removed = self.data.pop_front().unwrap();
        let _ = self.data.push_back(value);  // If we pop_front, push_back can never fail

        if removed == value {
            return self.sorted[31];
        }

        let remove_index = binary_search(&self.sorted, &removed);

        // If removed is larger than value than the remove_index must be
        // after the insert_index, allowing us to cut our search down
        let insert_index = {
            if removed > value {
                let sorted_slice = &self.sorted[..remove_index];
                binary_search(sorted_slice, &value)
            } else {
                let sorted_slice = &self.sorted[remove_index..];
                remove_index + binary_search(sorted_slice, &value)
            }
        };

        // shift the data between remove_index and insert_index so that the
        // value of remove_index is overwritten and the 'value' can be placed
        // in the gap between them

        if remove_index < insert_index {
            // Starting with a self.sorted of
            // [2, 3, 4, 5, 7, 8]
            // insert_and_calculate(6)
            // [2, 3, 4, 5, 7, 8] <- remove_index = 1, insert_index = 3
            // [2, 4, 5, 5, 7, 8]
            // [2, 4, 5, 6, 7, 8]

            scratch_space[remove_index + 1..insert_index]
                .copy_from_slice(&self.sorted[remove_index + 1..insert_index]);

            self.sorted[remove_index..insert_index - 1]
                .copy_from_slice(&scratch_space[remove_index + 1..insert_index]);

            self.sorted[insert_index - 1] = value;

        } else {
            // Starting with a self.sorted of
            // [2, 3, 4, 5, 7, 8, 9]
            // insert_and_calculate(6)
            // [2, 3, 4, 5, 7, 8, 9] <- remove_index = 5, insert_index = 3
            // [2, 3, 4, 5, 5, 7, 9] Shift values
            // [2, 3, 4, 6, 7, 8, 9] Insert value
            scratch_space[insert_index..remove_index]
                .copy_from_slice(&self.sorted[insert_index..remove_index]);

            self.sorted[insert_index + 1..remove_index + 1]
                .copy_from_slice(&scratch_space[insert_index..remove_index]);

            self.sorted[insert_index] = value;

        }

        let median = self.sorted[31];
        self.last_median = median;
        median
    }
}

fn binary_search<T>(t: &[T], x: &T) -> usize where T: Ord {
    binary_search_by(t, |p| p.cmp(x))
}

// A custom binary search that always returns a usize, showing where an item is or
// where an item can be inserted to preserve sorted order
// Since we have no use for differentiating between the two cases, a single usize
// is sufficient.
fn binary_search_by<T, F>(t: &[T], mut f: F) -> usize
    where F: FnMut(&T) -> Ordering
{
    let mut base = 0usize;
    let mut s = t;

    loop {
        let (head, tail) = s.split_at(s.len() >> 1);
        if tail.is_empty() {
            return base;
        }
        match f(&tail[0]) {
            Ordering::Less => {
                base += head.len() + 1;
                s = &tail[1..];
            }
            Ordering::Greater => s = head,
            Ordering::Equal => return base + head.len(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use xorshift::{Xoroshiro128, Rng, SeedableRng};
    use std::time::{SystemTime, UNIX_EPOCH};
    use std::time::Duration;

    const NANOS_PER_MILLI: u32 = 1000_000;
    const MILLIS_PER_SEC: u64 = 1000;

    pub fn millis(d: Duration) -> u64 {
        // A proper Duration will not overflow, because MIN and MAX are defined
        // such that the range is exactly i64 milliseconds.
        let secs_part = d.as_secs() * MILLIS_PER_SEC;
        let nanos_part = d.subsec_nanos() / NANOS_PER_MILLI;
        secs_part + nanos_part as u64
    }

    #[test]
    fn test_median_random() {
        let t = millis(SystemTime::now().duration_since(UNIX_EPOCH).unwrap());
        let mut rng = Xoroshiro128::from_seed(&[t, 71, 1223]);

        let mut median_tracker = StreamingMedian::new(123_000);
        for _ in 0..100_000 {
            median_tracker.insert_and_calculate(rng.gen());
        }

        for i in median_tracker.sorted.windows(2) {
            assert!(i[0] <= i[1]);
        }
    }

    #[test]
    fn test_median_ascending() {
        let mut median_tracker = StreamingMedian::new(123_000);

        let mut ascending_iter = 0..;
        for _ in 0..100_000 {
            median_tracker.insert_and_calculate(ascending_iter.next().unwrap());
        }

        for i in median_tracker.sorted.windows(2) {
            assert!(i[0] <= i[1]);
        }
    }

    #[test]
    fn test_median_descending() {
        let mut median_tracker = StreamingMedian::new(123_000);

        let mut ascending_iter = 200_000..;
        for _ in 0..100_000 {
            median_tracker.insert_and_calculate(ascending_iter.next().unwrap());
        }

        for i in median_tracker.sorted.windows(2) {
            assert!(i[0] <= i[1]);
        }
    }

    #[test]
    fn test_poison_absence() {
        let mut median_tracker = StreamingMedian::new(123_000);

        for _ in 0..64 {
            median_tracker.insert_and_calculate(1);
        }

        for i in median_tracker.sorted.iter() {
            assert_ne!(*i, 123_000);
        }
    }

    quickcheck! {
        fn maintains_sorted(default: u32, input: u32) -> bool {
            let mut median_tracker = StreamingMedian::new(default   );
            median_tracker.insert_and_calculate(input);

            for i in median_tracker.sorted.windows(2) {
                if i[0] > i[1] {
                    return false
                }
            }
            true
        }
    }
}