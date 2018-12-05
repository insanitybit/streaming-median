# streaming-median
A highly specialized data structure for calculating median values 

**Note that this library uses unsafe**

StreamingMedian only supports a very, very specific use case. When you want to
generate the median value, and update it every time. For example, if you want to
track the median time it takes for an operation to complete, updating it every time,
and only taking the last 64 timings into account.

It only supports medians over a range of 64 values. With int generics I could
make this arbitrary, but it suits my needs. PRs welcome if you want to add
generic integer support.

This crate uses `std::mem::uninitialized` for some scratch space. I've documented
why I believe it to be safe and I've added a number of tests.

Calculating median values over a random set of values, the worst case, is 74ns
on my laptop.
