#[macro_export]
macro_rules! deno_error {
  ($msg:expr) => {
    deno_core::error::CoreErrorKind::Io(::std::io::Error::new(
      std::io::ErrorKind::InvalidData,
      $msg,
    ))
    .into()
  };

  ($msg:expr, $kind:expr) => {
    deno_core::error::CoreErrorKind::Io(::std::io::Error::new($kind, $msg)).into()
  };
}
