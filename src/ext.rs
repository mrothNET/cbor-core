#[cfg(feature = "num-bigint")]
#[cfg_attr(docsrs, doc(cfg(feature = "num-bigint")))]
mod num_bigint;

#[cfg(feature = "crypto-bigint")]
#[cfg_attr(docsrs, doc(cfg(feature = "crypto-bigint")))]
mod crypto_bigint;

#[cfg(feature = "rug")]
#[cfg_attr(docsrs, doc(cfg(feature = "rug")))]
mod rug;

#[cfg(feature = "chrono")]
#[cfg_attr(docsrs, doc(cfg(feature = "chrono")))]
mod chrono;

#[cfg(feature = "time")]
#[cfg_attr(docsrs, doc(cfg(feature = "time")))]
mod time;

#[cfg(feature = "half")]
#[cfg_attr(docsrs, doc(cfg(feature = "half")))]
mod half;

#[cfg(feature = "serde")]
#[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
pub mod serde;
