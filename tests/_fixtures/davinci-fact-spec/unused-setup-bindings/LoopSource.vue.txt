<script setup lang="ts">
const colors = ['pink', 'red', 'cyan']
const unusedLabel = 'never read by the template'
const unusedCount = colors.length
</script>

<template>
  <span v-for="color in colors" :key="color">{{ color }}</span>
</template>
