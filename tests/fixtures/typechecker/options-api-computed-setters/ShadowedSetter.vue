<template>
  <div>{{ shadowedSetter }}</div>
</template>
<script lang="ts">
const undefined = (_value: string) => {}
export default {
  computed: {
    shadowedSetter: { get: () => 's', set: undefined },
  },
}
</script>
