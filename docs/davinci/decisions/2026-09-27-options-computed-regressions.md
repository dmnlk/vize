# Options API computed regression fixtures

Tracked in [#6879](https://github.com/ubugeeei-prod/vize/issues/6879) and
[#6921](https://github.com/ubugeeei-prod/vize/pull/6921). Preserve the original
writable-computed fix and its repeated/exported mixin precedence tests.

Extract computed writability and default-export classification into child
modules in a move-only commit. The parent returns below 350 lines; the moved
bodies preserve behavior. Keep the already-fixed recursion-path removal rather
than bypassing the unresolved review thread.

The private parse-sharing candidate derives default-export spans, writable
computed names and unresolved-extends status from one existing OXC script parse
and the same parsed Options object. Binding emission borrows those owned facts;
it adds no pipeline stage or serialization. Options facts are collected only
under the existing Options API template-binding gate, and setup-only rewrite
classification remains disabled. Existing rewrite guards, merge precedence,
repeated mixins, recursion-path removal and authored emission/mapping bodies are
preserved. The runtime-props helper's existing separate parse is outside this
small change. Joint-result tests cover export spans, repeated/exported mixin
precedence, unresolved extends, independent rewrite/binding gates and export
whitespace/comment controls that keep computed analysis unconditional. Source
compilation and exact diagnostic execution remain pending existing PR Actions.
The owner's later `5e0048c0` computed/prop-collision fix is preserved in a
separate private compatibility commit: its resolution body and regression test
remain byte-exact, with only the duplicate parse entry replaced by shared inputs.

The authored `tests/fixtures/typechecker/options-api-writable-computed` case
registers the existing SFC, compiler options, provenance and complete diagnostic
reference through the original Rust CLI integration test. A writable setter
produces no assignment diagnostic; the getter-only assignment retains its
severity, source position, TS2588 code and full message. The oracle compares
the entire diagnostic sequence and exit status, with no assertion-lint exemption.

This explicitly registered typechecker fixture does not enter automatic
`tests/_fixtures` L2/L3 sweeps. Their baselines are not changed without execution.
The future shared typechecker differential executor remains tracked by #6852
and #6879; native comparisons are zero. No native migration or fix-history
closure is claimed by this legacy regression.

Local preparation verifies formatting, preserved moved bodies and fixture
membership/hash metadata. The complete exact diagnostic oracle must be checked
by the existing source-built CLI/TSGO Actions gate after publication. Review
threads and latest-head checks remain required before auto-merge; no bot review
is dismissed or bypassed.
