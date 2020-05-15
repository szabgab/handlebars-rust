use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::iter::FromIterator;
use std::path::PathBuf;

use crate::context::Context;
use crate::error::RenderError;
use crate::helpers::HelperDef;
use crate::json::value::{to_json, ScopedJson};
use crate::registry::Registry;
use crate::render::{Helper, RenderContext};

use rhai::{Dynamic, Engine, Scope};

use serde_json::value::Value as Json;

pub struct ScriptHelper {
    script: String,
    engine: Engine,
}

pub struct FileScriptHelper {
    file_path: PathBuf,
    script: String,
    engine: Engine,
    cache: bool,
}

fn call_script_helper<'reg, 'rc>(
    h: &Helper<'reg, 'rc>,
    engine: &Engine,
    script: &str,
) -> Result<Option<ScopedJson<'reg, 'rc>>, RenderError> {
    let params: Vec<Dynamic> = h.params().iter().map(|p| convert(p.value())).collect();
    let hash: Dynamic = HashMap::from_iter(
        h.hash()
            .iter()
            .map(|(k, v)| ((*k).to_owned(), convert(v.value()))),
    )
    .into();

    let mut scope = Scope::new();
    scope.push_dynamic("params", params.into());
    scope.push_dynamic("hash", hash);

    let result = engine
        .eval_with_scope::<Dynamic>(&mut scope, script)
        .map_err(RenderError::from)?;
    let result_string = result.take_string().unwrap_or_else(|e| e.to_owned());

    Ok(Some(ScopedJson::Derived(to_json(result_string))))
}

impl HelperDef for ScriptHelper {
    fn call_inner<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        _reg: &'reg Registry<'reg>,
        _ctx: &'rc Context,
        _rc: &mut RenderContext<'reg, 'rc>,
    ) -> Result<Option<ScopedJson<'reg, 'rc>>, RenderError> {
        call_script_helper(h, &self.engine, &self.script)
    }
}

impl HelperDef for FileScriptHelper {
    fn call_inner<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        _reg: &'reg Registry<'reg>,
        _ctx: &'rc Context,
        _rc: &mut RenderContext<'reg, 'rc>,
    ) -> Result<Option<ScopedJson<'reg, 'rc>>, RenderError> {
        if self.cache {
            call_script_helper(h, &self.engine, &self.script)
        } else {
            let mut script = String::new();
            {
                let mut file = File::open(&self.file_path)?;
                file.read_to_string(&mut script)?;
            }
            call_script_helper(h, &self.engine, &script)
        }
    }
}

fn convert(j: &Json) -> Dynamic {
    match j {
        Json::Number(n) => Dynamic::from(n.clone()),
        Json::Bool(b) => Dynamic::from(*b),
        Json::Null => Dynamic::from(()),
        Json::String(s) => Dynamic::from(s.clone()),
        Json::Array(ref v) => Dynamic::from(v.clone()),
        Json::Object(ref o) => Dynamic::from(o.clone()),
    }
}
