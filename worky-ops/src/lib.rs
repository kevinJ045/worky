use deno_core::error::AnyError;
use deno_core::url::Url;
use deno_core::Extension;
use deno_permissions::{CheckedPath, OpenAccessKind, PermissionCheckError};
use std::{borrow::Cow, path::Path};

pub struct WorkyPermissions;

impl deno_fetch::FetchPermissions for WorkyPermissions {
  fn check_net_url(&mut self, _url: &Url, _api_name: &str) -> Result<(), PermissionCheckError> {
    Ok(())
  }
  fn check_open<'a>(
    &mut self,
    path: Cow<'a, Path>,
    _open_access: OpenAccessKind,
    _api_name: &str,
  ) -> Result<CheckedPath<'a>, PermissionCheckError> {
    Ok(CheckedPath::unsafe_new(path))
  }

  fn check_net_vsock(
    &mut self,
    _cid: u32,
    _port: u32,
    _api_name: &str,
  ) -> Result<(), PermissionCheckError> {
    Ok(())
  }
}

impl deno_web::TimersPermission for WorkyPermissions {
  fn allow_hrtime(&mut self) -> bool {
    true
  }
}

impl deno_websocket::WebSocketPermissions for WorkyPermissions {
  fn check_net_url(&mut self, _url: &Url, _api_name: &str) -> Result<(), PermissionCheckError> {
    Ok(())
  }
}

pub fn init_ops() -> Vec<Extension> {
  let blob_store = std::sync::Arc::new(deno_web::BlobStore::default());
  vec![
    deno_console::deno_console::init(),
    deno_webidl::deno_webidl::init(),
    deno_url::deno_url::init(),
    deno_web::deno_web::init::<WorkyPermissions>(blob_store, Default::default()),
    deno_fetch::deno_fetch::init::<WorkyPermissions>(deno_fetch::Options::default()),
    deno_websocket::deno_websocket::init::<WorkyPermissions>(Default::default(), None, None),
  ]
}
