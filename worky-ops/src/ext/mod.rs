#![allow(unused_variables)]
#![allow(clippy::derivable_impls)]
use deno_core::Extension;

trait ExtensionTrait<A> {
  fn init(options: A) -> Extension;

  fn set_esm(mut ext: Extension, is_snapshot: bool) -> Extension {
    if is_snapshot {
      ext.js_files = ::std::borrow::Cow::Borrowed(&[]);
      ext.esm_files = ::std::borrow::Cow::Borrowed(&[]);
      ext.esm_entry_point = ::std::option::Option::None;
    }
    ext
  }

  fn build(options: A, is_snapshot: bool) -> Extension {
    let ext = Self::init(options);
    Self::set_esm(ext, is_snapshot)
  }
}

pub mod console;
pub mod telemetry;
pub mod url;
pub mod web;
pub mod webidl;
pub mod worky;
pub mod kv;
pub mod secrets;
