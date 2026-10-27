# Unused setup bindings for the fact-spec corpus shard

The corpus shard of `tests/tooling/davinci-fact-spec-corpus.test.ts` compares
every Croquis fact group with its declarative spec's naive evaluator and fails
a group that compares zero facts. The hydrated real-project submodules carry no
genuinely unused `<script setup>` binding: the fourteen `UnusedBindings` facts
they used to contribute were bindings read only by a same-element `v-for`
source, which the drawer and the evaluator now both count as reads.

`LoopSource.vue` keeps the `unused-bindings` group non-degenerate with two
bindings nothing reads, and pins the loop-source case: `colors` is read by
`v-for="color in colors"` and must not be reported.
