//! Resolve the writability of local and inherited Options API computed members.

use oxc_ast::ast::{
    Declaration, Expression, ObjectExpression, ObjectPropertyKind, Program, PropertyKind, Statement,
};
use vize_carton::{FxHashMap, FxHashSet, String};

use super::{
    component_options_from_call, object_expression_from_expression, option_expression_property,
    option_object_property, property_key_name,
};

/// Names of `computed` members that declare a setter.
///
/// Vue exposes a `{ get, set }` computed (or a `get`/`set` accessor pair) as a
/// writable instance property, so a template assignment such as
/// `@input="ratio = $event"` is valid where a getter-only computed stays
/// read-only. Same-file `extends` / `mixins` objects contribute their members
/// with Vue's option precedence: a later source replaces an earlier one, and
/// the component's own declaration wins, so a local getter-only computed
/// shadows an inherited writable one.
pub(super) fn writable_computed_names<'a>(
    program: &'a Program<'a>,
    options: &'a ObjectExpression<'a>,
) -> FxHashSet<String> {
    let object_bindings = collect_object_expression_values(program);
    let mut seen = FxHashSet::default();
    resolved_computed_writability(options, &object_bindings, &mut seen)
        .into_iter()
        .filter_map(|(name, writable)| writable.then_some(name))
        .collect()
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
    if let Some(extends) = option_expression_property(options, "extends")
        && let Some(target) = resolve_options_object(extends, object_bindings)
    {
        resolved.extend(resolved_computed_writability(target, object_bindings, seen));
    }
    if let Some(Expression::ArrayExpression(mixins)) = option_expression_property(options, "mixins")
    {
        for element in mixins.elements.iter() {
            // Spreads and holes are not options objects.
            let Some(expression) = element.as_expression() else {
                continue;
            };
            if let Some(target) = resolve_options_object(expression, object_bindings) {
                resolved.extend(resolved_computed_writability(target, object_bindings, seen));
            }
        }
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
