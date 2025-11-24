use deno_core::Extension;

pub mod ext;
mod polyfill;

pub fn init_ops() -> Vec<Extension> {
  let mut extensions = vec![polyfill::polyfill_extension::init()];

  extensions.extend(ext::webidl::extensions(false));
  extensions.extend(ext::console::extensions(false));
  extensions.extend(ext::url::extensions(false));
  extensions.extend(ext::web::extensions(ext::web::WebOptions::default(), false));
  extensions.extend(ext::telemetry::extensions(false));
  // extensions.extend(ext::networking::extensions(false));

  extensions
}
