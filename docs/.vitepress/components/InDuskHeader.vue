<template>
  <div class="indusk-header">
    <span class="badge">{{ badge }}</span>
    <h1>{{ title }}</h1>
    <p v-if="subtitle">{{ subtitle }}</p>
  </div>
</template>

<script setup>
defineProps({
  title: { type: String, required: true },
  subtitle: { type: String, default: '' },
  badge: { type: String, default: '✨ InDusk' },
})
</script>
