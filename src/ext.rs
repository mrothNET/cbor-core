#[cfg(feature = "num-bigint")]
#[cfg_attr(docsrs, doc(cfg(feature = "num-bigint")))]
mod num_bigint;

#[cfg(feature = "crypto-bigint")]
#[cfg_attr(docsrs, doc(cfg(feature = "crypto-bigint")))]
mod crypto_bigint;

#[cfg(feature = "rug")]
#[cfg_attr(docsrs, doc(cfg(feature = "rug")))]
mod rug;
