//! How inherited, repeated and shadowing declarations resolve writability.

use super::options_api_declarations;

/// Vue merges `computed` with the component's own declaration winning over
/// `mixins`, and later mixins over earlier ones (and over `extends`). A local
/// getter-only computed therefore shadows an inherited writable one, and a
/// later mixin's getter-only computed shadows an earlier mixin's setter.
#[test]
fn local_and_later_declarations_shadow_inherited_writability() {
    let script = r#"const writableBase = {
    computed: {
        ratio: {
            get(): string {
                return '1'
            },
            set(_value: string) {},
        },
        title: {
            get(): string {
                return 't'
            },
            set(_value: string) {},
        },
    },
}

const readonlyOverride = {
    computed: {
        title() {
            return 't'
        },
    },
}

export default {
    mixins: [writableBase, readonlyOverride],
    computed: {
        ratio() {
            return '1'
        },
    },
}
"#;
    let declarations = options_api_declarations(script, r#"<div>{{ ratio }} {{ title }}</div>"#);
    assert_eq!(
        declarations,
        [
            "  const ratio: __VizeOptionsBinding<typeof __default__, \"ratio\"> = undefined as any;",
            "  const title: __VizeOptionsBinding<typeof __default__, \"title\"> = undefined as any;",
        ]
    );
}

/// A `set` accessor before its `get` accessor is the same writable pair.
#[test]
fn accessor_pair_is_writable_in_either_order() {
    let script = r#"export default {
    computed: {
        set first(_value: string) {},
        get first() {
            return 'a'
        },
    },
}
"#;
    let declarations = options_api_declarations(script, r#"<div @click="first = 'b'" />"#);
    assert_eq!(
        declarations,
        ["  var first: __VizeOptionsBinding<typeof __default__, \"first\"> = undefined as any;"]
    );
}

/// The same mixin listed again after another one that shadowed its setter
/// applies again at its later position, so the setter wins; and an exported
/// same-file mixin (`export const shared = { ... }`) is resolved like a
/// non-exported one.
#[test]
fn repeated_and_exported_mixins_apply_in_order() {
    let script = r#"export const shared = {
    computed: {
        ratio: {
            get(): string {
                return '1'
            },
            set(_value: string) {},
        },
        label: {
            get(): string {
                return 'l'
            },
            set(_value: string) {},
        },
    },
}

const readonlyOverride = {
    computed: {
        ratio() {
            return '1'
        },
        label() {
            return 'l'
        },
    },
}

export default {
    mixins: [shared, readonlyOverride, shared],
    computed: {
        label() {
            return 'l'
        },
    },
}
"#;
    let declarations = options_api_declarations(script, r#"<div>{{ ratio }} {{ label }}</div>"#);
    assert_eq!(
        declarations,
        [
            "  const label: __VizeOptionsBinding<typeof __default__, \"label\"> = undefined as any;",
            "  var ratio: __VizeOptionsBinding<typeof __default__, \"ratio\"> = undefined as any;",
        ]
    );
}

/// Vue resolves a template name from `data`, then `props`, and only then from
/// the context that exposes computed members. A writable computed that shares
/// a prop's name (its own, or one inherited from a mixin) never provides the
/// value, and the prop is read-only, so the binding stays `const`.
#[test]
fn writable_computed_sharing_a_prop_name_stays_const() {
    let script = r#"const withSize = {
    props: { size: String },
}

export default {
    mixins: [withSize],
    props: ['ratio'],
    computed: {
        ratio: {
            get(): string {
                return '1'
            },
            set(_value: string) {},
        },
        size: {
            get(): string {
                return 's'
            },
            set(_value: string) {},
        },
        free: {
            get(): string {
                return 'f'
            },
            set(_value: string) {},
        },
    },
}
"#;
    let declarations =
        options_api_declarations(script, r#"<div>{{ ratio }} {{ size }} {{ free }}</div>"#);
    assert_eq!(
        declarations,
        [
            "  var free: __VizeOptionsBinding<typeof __default__, \"free\"> = undefined as any;",
            "  const ratio: __VizeOptionsBinding<typeof __default__, \"ratio\"> = undefined as any;",
            "  const size: __VizeOptionsBinding<typeof __default__, \"size\"> = undefined as any;",
        ]
    );
}

/// `props` and `mixins` written through parentheses or TypeScript wrappers
/// (`(['a'] as const)`) resolve like plain arrays; a kebab-case prop shadows
/// its camelCase computed; and a descriptor whose `set` is statically absent
/// (`set: undefined`) or not callable stays read-only.
#[test]
fn wrapped_options_kebab_props_and_absent_setters() {
    let script = r#"const sizing = {
    computed: {
        ratio: {
            get(): string {
                return '1'
            },
            set(_value: string) {},
        },
    },
}

export default {
    mixins: ([sizing] as const),
    props: (['foo-bar', 'plain'] as const),
    computed: {
        fooBar: {
            get(): string {
                return 'f'
            },
            set(_value: string) {},
        },
        plain: {
            get(): string {
                return 'p'
            },
            set(_value: string) {},
        },
        absent: {
            get(): string {
                return 'a'
            },
            set: undefined,
        },
        literal: {
            get(): string {
                return 'l'
            },
            set: null,
        },
        arrow: {
            get: () => 'r',
            set: (_value: string) => {},
        },
    },
}
"#;
    let declarations = options_api_declarations(
        script,
        r#"<div>{{ ratio }} {{ fooBar }} {{ plain }} {{ absent }} {{ literal }} {{ arrow }}</div>"#,
    );
    assert_eq!(
        declarations,
        [
            "  const absent: __VizeOptionsBinding<typeof __default__, \"absent\"> = undefined as any;",
            "  var arrow: __VizeOptionsBinding<typeof __default__, \"arrow\"> = undefined as any;",
            "  const fooBar: __VizeOptionsBinding<typeof __default__, \"fooBar\"> = undefined as any;",
            "  const literal: __VizeOptionsBinding<typeof __default__, \"literal\"> = undefined as any;",
            "  const plain: __VizeOptionsBinding<typeof __default__, \"plain\"> = undefined as any;",
            "  var ratio: __VizeOptionsBinding<typeof __default__, \"ratio\"> = undefined as any;",
        ]
    );
}
