mod borrowing;
mod collections;
mod debug;
mod decode_options;
mod integers;
mod limits;
mod non_deterministic;
mod parse;
mod rundgren;
mod sequence;
mod simple_value;
mod value;

#[cfg(all(feature = "num-bigint", feature = "crypto-bigint", feature = "rug"))]
mod bigint_interop;

#[cfg(all(feature = "chrono", feature = "time", feature = "jiff"))]
mod time_interop;
