use super::ExtensionTrait;
use deno_core::{extension, Extension};
use std::sync::Arc;

mod options;
pub use options::WebOptions;

pub mod permissions;
pub(crate) use permissions::PermissionsContainer;
pub use permissions::{
  AllowlistWebPermissions, DefaultWebPermissions, SystemsPermissionKind, WebPermissions,
};

extension!(
    init_web,
    esm_entry_point = "ext:init_web/init_web.js",
    esm = [ dir "src/ext/web", "init_web.js", "init_fetch.js", "init_errors.js" ],
    options = {
        permissions: Arc<dyn WebPermissions>
    },
    state = |state, config| state.put(PermissionsContainer(config.permissions)),
);
impl ExtensionTrait<WebOptions> for init_web {
  fn init(options: WebOptions) -> Extension {
    init_web::init(options.permissions)
  }
}

impl ExtensionTrait<()> for deno_net::deno_net {
  fn init((): ()) -> Extension {
    deno_net::deno_net::init::<deno_permissions::PermissionsContainer>(None, None)
  }
}

impl ExtensionTrait<WebOptions> for deno_web::deno_web {
  fn init(options: WebOptions) -> Extension {
    deno_web::deno_web::init::<PermissionsContainer>(options.blob_store, options.base_url)
  }
}

impl ExtensionTrait<()> for deno_tls::deno_tls {
  fn init((): ()) -> Extension {
    deno_tls::deno_tls::init()
  }
}

impl ExtensionTrait<()> for deno_websocket::deno_websocket {
  fn init((): ()) -> Extension {
    deno_websocket::deno_websocket::init::<deno_permissions::PermissionsContainer>(
      "worky".to_string(),
      None,
      None,
    )
  }
}

impl ExtensionTrait<()> for deno_fetch::deno_fetch {
  fn init((): ()) -> Extension {
    deno_fetch::deno_fetch::init::<deno_permissions::PermissionsContainer>(
      deno_fetch::Options::default(),
    )
  }
}

pub fn extensions(options: WebOptions, is_snapshot: bool) -> Vec<Extension> {
  vec![
    deno_web::deno_web::build(options.clone(), is_snapshot),
    deno_net::deno_net::build((), is_snapshot),
    deno_websocket::deno_websocket::build((), is_snapshot),
    deno_fetch::deno_fetch::build((), is_snapshot),
    deno_tls::deno_tls::build((), is_snapshot),
    init_web::build(options.clone(), is_snapshot),
  ]
}
