//! Options API template-binding emission for the virtual TypeScript generator.

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    Argument, CallExpression, Declaration, ExportDefaultDeclarationKind, Expression,
    ObjectExpression, ObjectPropertyKind, Program, PropertyKey, PropertyKind, Statement,
};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType};
use vize_croquis::Croquis;
use vize_croquis::facts::used_component_name_list;

use super::options_api_support::is_safe_value_identifier;
use vize_carton::{CompactString, FxHashMap, FxHashSet, String};

mod variables;
pub(super) use variables::generate_options_api_variables;

fn unresolved_extends_template_names(
    summary: &Croquis,
    configured_globals: &FxHashSet<&str>,
    script: Option<&str>,
) -> Vec<String> {
    if !script.is_some_and(has_unresolved_extends) {
        return Vec::new();
    }

    let type_export_names: FxHashSet<&str> = summary
        .type_exports
        .iter()
        .map(|export| export.name.as_str())
        .collect();
    let used_components: FxHashSet<CompactString> =
        used_component_name_list(summary).into_iter().collect();
    let mut names = crate::virtual_ts::script_facts::undefined_refs(summary)
        .iter()
        .filter_map(|reference| {
            let name = reference.name.as_str();
            if crate::virtual_ts::script_facts::contains_binding(summary, name)
                || configured_globals.contains(name)
                || type_export_names.contains(name)
                || used_components.contains(name)
                || !is_safe_value_identifier(name)
            {
                return None;
            }
            Some(String::from(name))
        })
        .collect::<Vec<_>>();
    for expression in &summary.template_expressions {
        collect_unresolved_extends_expression_names(
            &mut names,
            expression.content.as_str(),
            summary,
            configured_globals,
            &type_export_names,
            &used_components,
        );
        if let Some(guard) = expression.vif_guard.as_ref() {
            collect_unresolved_extends_expression_names(
                &mut names,
                guard.as_str(),
                summary,
                configured_globals,
                &type_export_names,
                &used_components,
            );
        }
    }
    names.sort();
    names.dedup();
    names
}

fn has_unresolved_extends(script: &str) -> bool {
    if !script.contains("extends") || !script.contains("export default") {
        return false;
    }

    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, script, SourceType::ts()).parse();
    if parsed.panicked {
        return false;
    }

    let Some(options) = component_options_from_program(&parsed.program) else {
        return false;
    };
    let Some(extends) = option_expression_property(options, "extends") else {
        return false;
    };

    let object_bindings = collect_object_expression_bindings(&parsed.program);
    !is_resolved_options_target(extends, &object_bindings)
}

fn collect_object_expression_bindings<'a>(program: &'a Program<'a>) -> FxHashSet<&'a str> {
    let mut bindings = FxHashSet::default();
    for statement in program.body.iter() {
        let Statement::VariableDeclaration(declaration) = statement else {
            continue;
        };
        for declarator in declaration.declarations.iter() {
            let oxc_ast::ast::BindingPattern::BindingIdentifier(id) = &declarator.id else {
                continue;
            };
            let Some(init) = declarator.init.as_ref() else {
                continue;
            };
            if object_expression_from_expression(init).is_some() {
                bindings.insert(id.name.as_str());
            }
        }
    }
    bindings
}

fn is_resolved_options_target<'a>(
    expression: &'a Expression<'a>,
    object_bindings: &FxHashSet<&'a str>,
) -> bool {
    match expression {
        Expression::ObjectExpression(_) => true,
        Expression::Identifier(identifier) => object_bindings.contains(identifier.name.as_str()),
        Expression::ParenthesizedExpression(parenthesized) => {
            is_resolved_options_target(&parenthesized.expression, object_bindings)
        }
        Expression::TSAsExpression(ts_as) => {
            is_resolved_options_target(&ts_as.expression, object_bindings)
        }
        Expression::TSSatisfiesExpression(ts_satisfies) => {
            is_resolved_options_target(&ts_satisfies.expression, object_bindings)
        }
        Expression::TSNonNullExpression(ts_non_null) => {
            is_resolved_options_target(&ts_non_null.expression, object_bindings)
        }
        _ => false,
    }
}

fn collect_unresolved_extends_expression_names(
    names: &mut Vec<String>,
    expression: &str,
    summary: &Croquis,
    configured_globals: &FxHashSet<&str>,
    type_export_names: &FxHashSet<&str>,
    used_components: &FxHashSet<CompactString>,
) {
    for identifier in vize_croquis::drawer::extract_identifiers_oxc(expression) {
        let name = identifier.as_str();
        if crate::virtual_ts::script_facts::contains_binding(summary, name)
            || configured_globals.contains(name)
            || type_export_names.contains(name)
            || used_components.contains(name)
            || !is_safe_value_identifier(name)
        {
            continue;
        }
        names.push(String::from(name));
    }
}

/// Byte offsets locating the rewriteable shape of a `<script>` default export.
///
/// All fields are offsets into the parsed `script`. A single default export is
/// at most one of these (an SFC module has one default export), so at most one
/// field is `Some`.
#[derive(Default, Clone, Copy)]
pub(super) struct DefaultExportTargets {
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
pub(super) fn find_default_export_targets(script: &str) -> DefaultExportTargets {
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

pub(super) fn option_expression_property<'a>(
    object: &'a ObjectExpression<'a>,
    key_name: &str,
) -> Option<&'a Expression<'a>> {
    object.properties.iter().find_map(|property| {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            return None;
        };
        if property.computed || property_key_name(&property.key) != Some(key_name) {
            return None;
        }
        Some(&property.value)
    })
}

pub(super) fn component_options_from_program<'a>(
    program: &'a Program<'a>,
) -> Option<&'a ObjectExpression<'a>> {
    program.body.iter().find_map(|statement| {
        let Statement::ExportDefaultDeclaration(export) = statement else {
            return None;
        };
        component_options_from_export(&export.declaration)
    })
}

fn component_options_from_export<'a>(
    declaration: &'a ExportDefaultDeclarationKind<'a>,
) -> Option<&'a ObjectExpression<'a>> {
    match declaration {
        ExportDefaultDeclarationKind::ObjectExpression(object) => Some(object.as_ref()),
        ExportDefaultDeclarationKind::CallExpression(call) => component_options_from_call(call),
        ExportDefaultDeclarationKind::ParenthesizedExpression(parenthesized) => {
            component_options_from_expression(&parenthesized.expression)
        }
        ExportDefaultDeclarationKind::TSAsExpression(ts_as) => {
            component_options_from_expression(&ts_as.expression)
        }
        ExportDefaultDeclarationKind::TSSatisfiesExpression(ts_satisfies) => {
            component_options_from_expression(&ts_satisfies.expression)
        }
        ExportDefaultDeclarationKind::TSNonNullExpression(ts_non_null) => {
            component_options_from_expression(&ts_non_null.expression)
        }
        _ => None,
    }
}

fn component_options_from_expression<'a>(
    expression: &'a Expression<'a>,
) -> Option<&'a ObjectExpression<'a>> {
    match expression {
        Expression::ObjectExpression(object) => Some(object.as_ref()),
        Expression::CallExpression(call) => component_options_from_call(call),
        Expression::ParenthesizedExpression(parenthesized) => {
            component_options_from_expression(&parenthesized.expression)
        }
        Expression::TSAsExpression(ts_as) => component_options_from_expression(&ts_as.expression),
        Expression::TSSatisfiesExpression(ts_satisfies) => {
            component_options_from_expression(&ts_satisfies.expression)
        }
        Expression::TSNonNullExpression(ts_non_null) => {
            component_options_from_expression(&ts_non_null.expression)
        }
        _ => None,
    }
}

fn component_options_from_call<'a>(
    call: &'a CallExpression<'a>,
) -> Option<&'a ObjectExpression<'a>> {
    if !is_define_component_callee(&call.callee) {
        return None;
    }
    let first = call.arguments.first()?;
    match first {
        Argument::ObjectExpression(object) => Some(object.as_ref()),
        Argument::CallExpression(call) => component_options_from_call(call),
        Argument::ParenthesizedExpression(parenthesized) => {
            component_options_from_expression(&parenthesized.expression)
        }
        Argument::TSAsExpression(ts_as) => component_options_from_expression(&ts_as.expression),
        Argument::TSSatisfiesExpression(ts_satisfies) => {
            component_options_from_expression(&ts_satisfies.expression)
        }
        Argument::TSNonNullExpression(ts_non_null) => {
            component_options_from_expression(&ts_non_null.expression)
        }
        _ => None,
    }
}

fn is_define_component_callee(callee: &Expression<'_>) -> bool {
    match callee {
        Expression::Identifier(callee) => {
            matches!(callee.name.as_str(), "defineComponent" | "_defineComponent")
        }
        Expression::StaticMemberExpression(member) => {
            matches!(
                member.property.name.as_str(),
                "defineComponent" | "_defineComponent"
            )
        }
        _ => false,
    }
}

pub(super) fn option_object_property<'a>(
    object: &'a ObjectExpression<'a>,
    key_name: &str,
) -> Option<&'a ObjectExpression<'a>> {
    object.properties.iter().find_map(|property| {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            return None;
        };
        if property.computed || property_key_name(&property.key) != Some(key_name) {
            return None;
        }
        object_expression_from_expression(&property.value)
    })
}

fn object_expression_from_expression<'a>(
    expression: &'a Expression<'a>,
) -> Option<&'a ObjectExpression<'a>> {
    match expression {
        Expression::ObjectExpression(object) => Some(object.as_ref()),
        Expression::ParenthesizedExpression(parenthesized) => {
            object_expression_from_expression(&parenthesized.expression)
        }
        Expression::TSAsExpression(ts_as) => object_expression_from_expression(&ts_as.expression),
        Expression::TSSatisfiesExpression(ts_satisfies) => {
            object_expression_from_expression(&ts_satisfies.expression)
        }
        Expression::TSNonNullExpression(ts_non_null) => {
            object_expression_from_expression(&ts_non_null.expression)
        }
        _ => None,
    }
}

pub(super) fn property_key_name<'a>(key: &'a PropertyKey<'a>) -> Option<&'a str> {
    match key {
        PropertyKey::StaticIdentifier(identifier) => Some(identifier.name.as_str()),
        PropertyKey::StringLiteral(string) => Some(string.value.as_str()),
        _ => None,
    }
}

pub(super) fn source_slice(script: &str, span: oxc_span::Span) -> Option<&str> {
    script.get(span.start as usize..span.end as usize)
}

pub(super) fn safe_identifier(name: &str) -> String {
    let mut result = String::default();
    for (index, ch) in name.chars().enumerate() {
        if (index == 0 && (ch.is_ascii_alphabetic() || ch == '_' || ch == '$'))
            || (index > 0 && (ch.is_ascii_alphanumeric() || ch == '_' || ch == '$'))
        {
            result.push(ch);
        } else {
            result.push('_');
        }
    }
    if result.is_empty() {
        result.push('_');
    }
    result
}

/// Names of `computed` members that declare a setter.
///
/// Vue exposes a `{ get, set }` computed (or a `get`/`set` accessor pair) as a
/// writable instance property, so a template assignment such as
/// `@input="ratio = $event"` is valid where a getter-only computed stays
/// read-only. Same-file `extends` / `mixins` objects contribute their members
/// with Vue's option precedence: a later source replaces an earlier one, and
/// the component's own declaration wins, so a local getter-only computed
/// shadows an inherited writable one.
pub(super) fn writable_computed_names(script: &str) -> FxHashSet<String> {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, script, SourceType::ts()).parse();
    if parsed.panicked {
        return FxHashSet::default();
    }
    let Some(options) = component_options_from_program(&parsed.program) else {
        return FxHashSet::default();
    };
    let object_bindings = collect_object_expression_values(&parsed.program);
    let mut seen = FxHashSet::default();
    let resolved = resolved_computed_writability(options, &object_bindings, &mut seen);
    // Vue reads a template name from `data`, then `props`, and only then from
    // the context that exposes computed members, so a computed sharing a
    // prop's name never provides the value: the prop does, and it is
    // read-only. (`data` sharing the name is writable in its own right.)
    let props = resolved_prop_names(options, &object_bindings, &mut seen);
    resolved
        .into_iter()
        .filter_map(|(name, writable)| (writable && !props.contains(&name)).then_some(name))
        .collect()
}

/// Every prop name an options object resolves to through `extends`, `mixins`
/// and its own `props`, in array (`['a']`) or object (`{ a: ... }`) form.
fn resolved_prop_names<'a>(
    options: &'a ObjectExpression<'a>,
    object_bindings: &FxHashMap<&'a str, &'a ObjectExpression<'a>>,
    seen: &mut FxHashSet<u32>,
) -> FxHashSet<String> {
    let mut props = FxHashSet::default();
    if !seen.insert(options.span.start) {
        return props;
    }
    for source in inherited_options_objects(options, object_bindings) {
        props.extend(resolved_prop_names(source, object_bindings, seen));
    }
    match option_expression_property(options, "props") {
        Some(Expression::ArrayExpression(names)) => {
            for element in names.elements.iter() {
                if let Some(Expression::StringLiteral(name)) = element.as_expression() {
                    props.insert(String::from(name.value.as_str()));
                }
            }
        }
        Some(expression) => {
            if let Some(object) = object_expression_from_expression(expression) {
                for property in object.properties.iter() {
                    let ObjectPropertyKind::ObjectProperty(property) = property else {
                        continue;
                    };
                    if property.computed {
                        continue;
                    }
                    if let Some(name) = property_key_name(&property.key) {
                        props.insert(String::from(name));
                    }
                }
            }
        }
        None => {}
    }
    seen.remove(&options.span.start);
    props
}

/// The options objects an object inherits from, in Vue's merge order:
/// `extends` first, then each `mixins` entry.
fn inherited_options_objects<'a>(
    options: &'a ObjectExpression<'a>,
    object_bindings: &FxHashMap<&'a str, &'a ObjectExpression<'a>>,
) -> Vec<&'a ObjectExpression<'a>> {
    let mut sources = Vec::new();
    if let Some(extends) = option_expression_property(options, "extends")
        && let Some(target) = resolve_options_object(extends, object_bindings)
    {
        sources.push(target);
    }
    if let Some(Expression::ArrayExpression(mixins)) = option_expression_property(options, "mixins")
    {
        for element in mixins.elements.iter() {
            // Spreads and holes are not options objects.
            let Some(expression) = element.as_expression() else {
                continue;
            };
            if let Some(target) = resolve_options_object(expression, object_bindings) {
                sources.push(target);
            }
        }
    }
    sources
}

/// Every `computed` name an options object resolves to, with whether it is
/// writable, after applying `extends`, then each `mixins` entry in order, then
/// the object's own `computed`. Each later source replaces the entry of an
/// earlier one, which is how Vue merges the option.
fn resolved_computed_writability<'a>(
    options: &'a ObjectExpression<'a>,
    object_bindings: &FxHashMap<&'a str, &'a ObjectExpression<'a>>,
    seen: &mut FxHashSet<u32>,
) -> FxHashMap<String, bool> {
    let mut resolved = FxHashMap::default();
    // `seen` holds the objects on the current resolution path: a cycle stops,
    // but the same mixin listed twice (or reached through two chains) is
    // applied at each position, as Vue does.
    if !seen.insert(options.span.start) {
        return resolved;
    }
    for source in inherited_options_objects(options, object_bindings) {
        resolved.extend(resolved_computed_writability(source, object_bindings, seen));
    }
    for (name, writable) in local_computed_writability(options) {
        resolved.insert(name, writable);
    }
    seen.remove(&options.span.start);
    resolved
}

/// The `computed` members an options object declares itself, with whether
/// each is writable. A `get`/`set` accessor pair declares the name twice, so
/// the entries are folded per name: any setter makes the name writable.
fn local_computed_writability<'a>(options: &'a ObjectExpression<'a>) -> Vec<(String, bool)> {
    let mut local: Vec<(String, bool)> = Vec::new();
    let Some(computed) = option_object_property(options, "computed") else {
        return local;
    };
    for property in computed.properties.iter() {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            continue;
        };
        if property.computed {
            continue;
        }
        let Some(name) = property_key_name(&property.key) else {
            continue;
        };
        let writable = match property.kind {
            PropertyKind::Set => true,
            PropertyKind::Get => false,
            PropertyKind::Init => object_expression_from_expression(&property.value)
                .is_some_and(|descriptor| option_expression_property(descriptor, "set").is_some()),
        };
        match local.iter_mut().find(|(existing, _)| existing == name) {
            Some((_, existing_writable)) => *existing_writable |= writable,
            None => local.push((String::from(name), writable)),
        }
    }
    local
}

/// Module-scope `const name = { ... }` objects, exported or not, the
/// same-file targets a `mixins` / `extends` entry can name.
fn collect_object_expression_values<'a>(
    program: &'a Program<'a>,
) -> FxHashMap<&'a str, &'a ObjectExpression<'a>> {
    let mut bindings = FxHashMap::default();
    for statement in program.body.iter() {
        let declaration = match statement {
            Statement::VariableDeclaration(declaration) => declaration,
            Statement::ExportNamedDeclaration(export) => {
                let Some(Declaration::VariableDeclaration(declaration)) =
                    export.declaration.as_ref()
                else {
                    continue;
                };
                declaration
            }
            _ => continue,
        };
        for declarator in declaration.declarations.iter() {
            let oxc_ast::ast::BindingPattern::BindingIdentifier(id) = &declarator.id else {
                continue;
            };
            let Some(init) = declarator.init.as_ref() else {
                continue;
            };
            if let Some(object) = resolve_options_object(init, &FxHashMap::default()) {
                bindings.insert(id.name.as_str(), object);
            }
        }
    }
    bindings
}

/// The options object an `extends` / `mixins` entry names: an inline object,
/// a same-file `const` bound to one, or a `defineComponent({ ... })` call
/// around one.
fn resolve_options_object<'a>(
    expression: &'a Expression<'a>,
    object_bindings: &FxHashMap<&'a str, &'a ObjectExpression<'a>>,
) -> Option<&'a ObjectExpression<'a>> {
    match expression {
        Expression::Identifier(identifier) => {
            object_bindings.get(identifier.name.as_str()).copied()
        }
        Expression::CallExpression(call) => component_options_from_call(call),
        _ => object_expression_from_expression(expression),
    }
}
