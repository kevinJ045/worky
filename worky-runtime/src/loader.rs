use anyhow::Result;
use deno_core::ModuleLoader;
use deno_core::ModuleSource;
use deno_core::ModuleSpecifier;
use deno_core::ModuleType;
use deno_core::ResolutionKind;
use std::path::Path;

pub struct FsModuleLoader;

impl ModuleLoader for FsModuleLoader {
  fn resolve(
    &self,
    specifier: &str,
    referrer: &str,
    _kind: ResolutionKind,
  ) -> Result<ModuleSpecifier, anyhow::Error> {
    let url = deno_core::resolve_import(specifier, referrer)?;
    Ok(url)
  }

  fn load(
    &self,
    module_specifier: &ModuleSpecifier,
    _maybe_referrer: Option<&ModuleSpecifier>,
    _is_dyn_import: bool,
    _requested_module_type: deno_core::RequestedModuleType,
  ) -> deno_core::ModuleLoadResponse {
    let module_specifier = module_specifier.clone();

    let fut = async move {
      let path = module_specifier
        .to_file_path()
        .map_err(|_| anyhow::anyhow!("Only file:// URLs are supported"))?;

      let code = tokio::fs::read_to_string(&path).await?;

      let module_type = if path.extension().map_or(false, |ext| ext == "json") {
        ModuleType::Json
      } else {
        ModuleType::JavaScript
      };

      Ok(ModuleSource::new(
        module_type,
        deno_core::ModuleSourceCode::String(code.into()),
        &module_specifier,
        None,
      ))
    };

    deno_core::ModuleLoadResponse::Async(Box::pin(fut))
  }
}
