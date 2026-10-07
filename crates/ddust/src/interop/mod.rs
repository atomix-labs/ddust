//! Other crates' traits, each crate behind its own feature.

#[cfg(feature = "arbitrary")]
mod arbitrary;
#[cfg(feature = "bytemuck")]
mod bytemuck;
#[cfg(feature = "defmt")]
mod defmt;
#[cfg(feature = "num-traits-02")]
mod num_traits;
#[cfg(feature = "proptest")]
mod proptest;
#[cfg(feature = "zerocopy-08")]
mod zerocopy;
