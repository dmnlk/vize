# Options API computed regression fixtures

Tracked in [#6879](https://github.com/ubugeeei-prod/vize/issues/6879) and
[#6921](https://github.com/ubugeeei-prod/vize/pull/6921). Preserve the original
writable-computed fix and its repeated/exported mixin precedence tests.

Extract computed writability and default-export classification into child
modules in a move-only commit. The parent returns below 350 lines; the moved
bodies preserve behavior. Keep the already-fixed recursion-path removal rather
than bypassing the unresolved review thread.

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
