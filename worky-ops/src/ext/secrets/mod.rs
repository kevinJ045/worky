use crate::ext::ExtensionTrait;
use deno_core::extension;
use deno_core::op2;
use deno_core::Extension;
use deno_core::OpState;
use std::collections::HashMap;

#[derive(Clone)]
pub struct SecretsOptions {
  pub secrets: HashMap<String, String>,
}

extension!(
    worky_secrets,
    ops = [op_secret_get],
    esm_entry_point = "ext:worky_secrets/01_secrets.js",
    esm = [ dir "src/ext/secrets", "01_secrets.js" ],
    options = {
        opts: SecretsOptions
    },
    state = |state, config| {
        state.put(config.opts.secrets);
    }
);

impl ExtensionTrait<SecretsOptions> for worky_secrets {
  fn init(opts: SecretsOptions) -> Extension {
    worky_secrets::init(opts)
  }
}

#[op2]
#[string]
pub fn op_secret_get(#[state] state: &mut OpState, #[string] key: String) -> Option<String> {
  let secrets = state.borrow::<HashMap<String, String>>();
  secrets.get(&key).cloned()
}
