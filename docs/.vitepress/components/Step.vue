<template>
  <div class="vp-step">
    <div class="step-title">{{ title }}</div>
    <div class="step-content">
      <slot></slot>
    </div>
  </div>
</template>

<script setup>
defineProps({
  title: { type: String, required: true },
})
</script>
