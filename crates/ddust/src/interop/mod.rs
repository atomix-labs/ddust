//! Other crates' traits, each crate behind its own feature.

#[cfg(feature = "arbitrary")]
mod arbitrary;
#[cfg(feature = "defmt")]
mod defmt;
#[cfg(feature = "num-traits-02")]
mod num_traits;
#[cfg(feature = "proptest")]
mod proptest;
