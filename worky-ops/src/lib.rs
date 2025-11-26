use deno_core::Extension;

pub mod ext;
pub use ext::worky::WorkyInitOptions;

pub fn init_ops(opts: WorkyInitOptions) -> Vec<Extension> {
  let mut extensions = vec![ext::worky::worky_js::init(opts)];

  extensions.extend(ext::webidl::extensions(false));
  extensions.extend(ext::console::extensions(false));
  extensions.extend(ext::url::extensions(false));
  extensions.extend(ext::web::extensions(ext::web::WebOptions::default(), false));
  extensions.extend(ext::telemetry::extensions(false));
  // extensions.extend(ext::networking::extensions(false));

  extensions
}
