<template>
  <div>{{ hoistedSetter }}</div>
</template>
<script lang="ts">
if (true) {
  var undefined = (_value: string) => {}
}
export default {
  computed: {
    hoistedSetter: { get: () => 's', set: undefined },
  },
}
</script>
