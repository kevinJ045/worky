use deno_core::{extension, op2};

extension!(
  polyfill_extension,
  ops = [op_tls_peer_certificate],
  esm = [ dir "src/ext/polyfill", "init_utils.js" ]
);

#[op2]
#[string]
fn op_tls_peer_certificate() -> String {
  "".to_string()
}
