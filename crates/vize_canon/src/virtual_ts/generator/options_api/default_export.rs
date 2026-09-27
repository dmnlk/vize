//! Default-export span classification for Options API virtual TypeScript.

use oxc_allocator::Allocator;
use oxc_ast::ast::{ExportDefaultDeclarationKind, Statement};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType};

/// Byte offsets locating the rewriteable shape of a `<script>` default export.
///
/// All fields are offsets into the parsed `script`. A single default export is
/// at most one of these (an SFC module has one default export), so at most one
/// field is `Some`.
#[derive(Default, Clone, Copy)]
pub(in crate::virtual_ts::generator) struct DefaultExportTargets {
    /// A plain object-literal default export (`export default { ... }`) — the
    /// Options API shape — as `(export_start, object_start, object_end)`. Used
    /// to wrap the object in `defineComponent` so `this` in computed/methods
    /// gets Vue's instance typing. Anything else (already-wrapped
    /// `defineComponent({...})`, identifiers, calls, `as`/`satisfies`) stays
    /// `None` so only the bare options object is wrapped.
    pub object: Option<(usize, usize, usize)>,
    /// A class-declaration default export (`export default class Foo {}`, the
    /// class-component shape — vue-class-component / vue-property-decorator) as
    /// `(export_start, class_start, class_end, name_start, name_end)`.
    /// `export_start..class_start` is the `export default ` keyword (stripped);
    /// `class_start..class_end` is the class declaration; `name_start..name_end`
    /// is the class identifier. Decorators written before `export default` sit
    /// ahead of `export_start`; decorators after it fall inside the class span —
    /// so stripping only the keyword run keeps `@Component()` on a real class
    /// declaration either way (the line-based fallback would move it onto a
    /// `const`, which TypeScript rejects with TS1206). Anonymous default classes
    /// stay `None` (no name to alias by) and fall through to the generic
    /// `expr` rewrite below.
    pub class: Option<(usize, usize, usize, usize, usize)>,
    /// Any other default-export shape, rewritten to a bare
    /// `const __default__ = <expr>` at module scope, as
    /// `(export_start, expr_start, expr_end)`. Covers
    /// `export default defineComponent({...})`, identifiers, parenthesized /
    /// `as` / `satisfies` expressions, anonymous classes/functions, and
    /// `export default{` with no space — including multi-line / awkwardly
    /// formatted variants. `export_start..expr_start` is the `export default`
    /// keyword run that is dropped; `expr_start..expr_end` is the exported
    /// expression copied verbatim. This is the span-based replacement for the
    /// former line-scanning fallback, so it is only populated when neither
    /// `object` nor `class` applies.
    pub expr: Option<(usize, usize, usize)>,
}

/// Classify a `<script>` default export in a single parse. Parsing once keeps
/// the virtual-TS hot path free of a second full OXC parse per plain-`<script>`
/// component.
pub(in crate::virtual_ts::generator) fn find_default_export_targets(
    script: &str,
) -> DefaultExportTargets {
    let mut targets = DefaultExportTargets::default();
    if !script.contains("export default") {
        return targets;
    }
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, script, SourceType::ts()).parse();
    if parsed.panicked {
        return targets;
    }
    for statement in parsed.program.body.iter() {
        let Statement::ExportDefaultDeclaration(export) = statement else {
            continue;
        };
        match &export.declaration {
            ExportDefaultDeclarationKind::ObjectExpression(object) => {
                let object_span = object.span();
                targets.object = Some((
                    export.span.start as usize,
                    object_span.start as usize,
                    object_span.end as usize,
                ));
            }
            ExportDefaultDeclarationKind::ClassDeclaration(class) if let Some(id) = &class.id => {
                targets.class = Some((
                    export.span.start as usize,
                    class.span.start as usize,
                    class.span.end as usize,
                    id.span.start as usize,
                    id.span.end as usize,
                ));
            }
            // Every other default-export shape (already-wrapped
            // `defineComponent(...)`, identifiers, `as`/`satisfies`,
            // anonymous classes/functions, ...) is rewritten verbatim to a
            // bare `const __default__ = <expr>` using the declaration span.
            // Slicing on these AST offsets is correct regardless of source
            // formatting (`export default{` with no space, multi-line calls),
            // which the previous line scanner mishandled.
            other => {
                let declaration_span = other.span();
                targets.expr = Some((
                    export.span.start as usize,
                    declaration_span.start as usize,
                    declaration_span.end as usize,
                ));
            }
        }
        // A module has a single default export; stop at the first one.
        break;
    }
    targets
}
