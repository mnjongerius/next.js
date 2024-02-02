use anyhow::Result;
use async_trait::async_trait;
use next_custom_transforms::transforms::react_server_components::*;
use swc_core::{
    common::{util::take::Take, FileName},
    ecma::{
        ast::{Module, Program},
        visit::FoldWith,
    },
};
use turbo_tasks::Vc;
use turbo_tasks_fs::FileSystemPath;
use turbopack_binding::turbopack::{
    ecmascript::{CustomTransformer, EcmascriptInputTransform, TransformContext},
    turbopack::module_options::{ModuleRule, ModuleRuleEffect},
};

use super::{get_ecma_transform_rule, module_rule_match_js_no_url};
use crate::next_config::NextConfig;

/// Returns a rule which applies the Next.js react server components transform.
pub async fn get_next_react_server_components_transform_rule(
    next_config: Vc<NextConfig>,
    app_dir: Option<Vc<FileSystemPath>>,
    is_react_server_layer: bool,
    chain_transformer: Option<Box<dyn CustomTransformer + Send + Sync>>,
) -> Result<ModuleRule> {
    /*
    let transformer = EcmascriptInputTransform::Plugin(Vc::cell(transformer as _));
    let (prepend, append) = if prepend {
        (Vc::cell(vec![transformer]), Vc::cell(vec![]))
    } else {
        (Vc::cell(vec![]), Vc::cell(vec![transformer]))
    }; */

    let transformers = vec![
        Some(Box::new(NextJsReactServerComponents::new(
            is_react_server_layer,
            app_dir,
        )) as _),
        chain_transformer,
    ]
    .into_iter()
    .flatten()
    .map(|v| EcmascriptInputTransform::Plugin(Vc::cell(v)))
    .collect();

    Ok(ModuleRule::new(
        module_rule_match_js_no_url(*next_config.mdx_rs().await?),
        vec![ModuleRuleEffect::ExtendEcmascriptTransforms {
            prepend: Vc::cell(transformers),
            append: Vc::cell(vec![]),
        }],
    ))

    /*
    Ok(get_ecma_transform_rule(
        Box::new(NextJsReactServerComponents::new(
            is_react_server_layer,
            app_dir,
        )),
        enable_mdx_rs,
        true,
    )) */
}

#[derive(Debug)]
struct NextJsReactServerComponents {
    is_react_server_layer: bool,
    app_dir: Option<Vc<FileSystemPath>>,
}

impl NextJsReactServerComponents {
    fn new(is_react_server_layer: bool, app_dir: Option<Vc<FileSystemPath>>) -> Self {
        Self {
            is_react_server_layer,
            app_dir,
        }
    }
}

#[async_trait]
impl CustomTransformer for NextJsReactServerComponents {
    async fn transform(&self, program: &mut Program, ctx: &TransformContext<'_>) -> Result<()> {
        let p = std::mem::replace(program, Program::Module(Module::dummy()));
        let file_name = if ctx.file_path_str.is_empty() {
            FileName::Anon
        } else {
            FileName::Real(ctx.file_path_str.into())
        };

        let mut visitor = server_components(
            file_name,
            Config::WithOptions(Options {
                is_react_server_layer: self.is_react_server_layer,
            }),
            ctx.comments,
            match self.app_dir {
                None => None,
                Some(path) => Some(path.await?.path.clone().into()),
            },
        );

        *program = p.fold_with(&mut visitor);

        Ok(())
    }
}
