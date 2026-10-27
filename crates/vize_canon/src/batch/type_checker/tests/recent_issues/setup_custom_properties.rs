//! A member a global mixin or plugin declares through
//! `ComponentCustomProperties` resolves in a `<script setup>` template with its
//! declared type, as it does on `vue-tsc`'s template context; a name nothing
//! declares keeps the instance diagnostic, once per authored occurrence.

use std::path::Path;

use super::super::{
    BatchTypeChecker, create_project_case, relative_path, resolve_test_tsgo_binary,
};
use crate::batch::{BatchTypeCheckerOptions, TypeChecker};
use vize_l0::{String, cstr};

/// A global mixin's members, declared the way a plugin declares them.
const AUGMENTATION: &str = "import 'vue';\n\
declare module 'vue' {\n\
  interface ComponentCustomProperties {\n\
    isSP: boolean;\n\
    appName: string;\n\
  }\n\
}\n";

const BOX: &str = r#"<template>
  <div><slot name="text" /></div>
</template>

<script lang="ts">
import { defineComponent } from 'vue'
export default defineComponent({ name: 'Box' })
</script>
"#;

const SETUP: &str = r#"<script setup lang="ts">
import Box from './Box.vue'
const label = 'x'
</script>

<template>
  <span :title="isSP ? 'a' : 'b'">{{ appName.toUpperCase() }} {{ label }}</span>
  <Box><template #text><i>{{ isSP }}</i></template></Box>
  <b>{{ isSP.toFixed() }}</b>
  <u>{{ missing }} {{ missing }}</u>
</template>
"#;

const OPTIONS: &str = r#"<template>
  <div>
    <span>{{ appName.toUpperCase() }}</span>
    <Box><template #text><i v-for="i in (isSP ? 5 : 4)" :key="i">x</i></template></Box>
    <b>{{ missing }}</b>
  </div>
</template>

<script lang="ts">
import { defineComponent } from 'vue'
import Box from './Box.vue'

export default defineComponent({ components: { Box } })
</script>
"#;

#[test]
fn custom_properties_resolve_in_script_setup_templates() {
    if resolve_test_tsgo_binary().is_none() {
        return;
    }
    let project_root = create_project_case(
        "setup-custom-properties",
        &[
            ("src/mixin.d.ts", AUGMENTATION),
            ("src/Box.vue", BOX),
            ("src/Setup.vue", SETUP),
        ],
    );
    let diagnostics = project_diagnostics(&project_root, false);
    let _ = std::fs::remove_dir_all(&project_root);

    assert_eq!(
        diagnostics,
        vec![
            (
                "src/Setup.vue".into(),
                Some(2339),
                "10:23: Property 'missing' does not exist on the component instance.".into()
            ),
            (
                "src/Setup.vue".into(),
                Some(2339),
                "10:9: Property 'missing' does not exist on the component instance.".into()
            ),
            (
                "src/Setup.vue".into(),
                Some(2339),
                "9:14: Property 'toFixed' does not exist on type 'boolean'.".into()
            ),
        ]
    );
}

#[test]
fn custom_properties_resolve_in_options_api_templates() {
    if resolve_test_tsgo_binary().is_none() {
        return;
    }
    let project_root = create_project_case(
        "options-custom-properties",
        &[
            ("src/mixin.d.ts", AUGMENTATION),
            ("src/Box.vue", BOX),
            ("src/Options.vue", OPTIONS),
        ],
    );
    let diagnostics = project_diagnostics(&project_root, true);
    let _ = std::fs::remove_dir_all(&project_root);

    assert_eq!(
        diagnostics,
        vec![(
            "src/Options.vue".into(),
            Some(2339),
            "5:11: Property 'missing' does not exist on type 'ComponentPublicInstance<{}>'.".into()
        )]
    );
}

/// The caller already gated on a resolvable checker binary, so every failure
/// past that point is a real regression and panics instead of skipping.
fn project_diagnostics(
    project_root: &Path,
    options_api: bool,
) -> Vec<(String, Option<u32>, String)> {
    let mut checker =
        BatchTypeChecker::with_options(project_root, BatchTypeCheckerOptions::default())
            .expect("batch checker should initialize");
    if options_api {
        checker.enable_options_api();
    }
    checker.scan_project().expect("project scan should succeed");
    let result = checker.check_project().expect("project check should run");

    let mut snapshot: Vec<_> = result
        .diagnostics
        .into_iter()
        .map(|diagnostic| {
            (
                relative_path(project_root, &diagnostic.file),
                diagnostic.code,
                cstr!(
                    "{}:{}: {}",
                    diagnostic.line + 1,
                    diagnostic.column + 1,
                    diagnostic.message
                ),
            )
        })
        .collect();
    snapshot.sort();
    snapshot
}
