use deno_core::Extension;

pub mod ext;
pub use ext::worky::WorkyInitOptions;

pub fn init_ops(opts: WorkyInitOptions) -> Vec<Extension> {
  let secrets = opts.secrets.clone();
  let kvdb = opts.kv_db.clone();
  let mut extensions = vec![ext::worky::worky_js::init(opts)];

  extensions.extend(ext::webidl::extensions(false));
  extensions.extend(ext::console::extensions(false));
  extensions.extend(ext::url::extensions(false));
  extensions.extend(ext::web::extensions(
    ext::web::WebOptions {
      permissions: std::sync::Arc::new(ext::web::permissions::RestrictedWebPermissions),
      ..Default::default()
    },
    false,
  ));
  extensions.extend(ext::telemetry::extensions(false));
  extensions.extend(vec![ext::kv::worky_kv::init(ext::kv::KvOptions {
    db: kvdb,
  })]);
  extensions.extend(vec![ext::secrets::worky_secrets::init(
    ext::secrets::SecretsOptions { secrets: secrets },
  )]);
  // extensions.extend(ext::networking::extensions(false));

  extensions
}
