//! The book's listings, each a test, so a page shows code that compiles and runs.
//!
//! A page includes a listing by the anchor around it; a chapter's listings are in the module named
//! for it.

// Overflow decided by the profile's `overflow-checks`, as the crate decides it: the listing of a
// panic past the range runs only where the checks are on.
#![feature(cfg_overflow_checks)]

#[cfg(test)]
mod arithmetic;
#[cfg(test)]
mod decimals;
#[cfg(test)]
mod floats;
#[cfg(test)]
mod getting_started;
#[cfg(test)]
mod interop;
#[cfg(test)]
mod rounding;
#[cfg(test)]
mod run_time;
#[cfg(test)]
mod serialization;
#[cfg(test)]
mod test_inputs;
#[cfg(test)]
mod text;
