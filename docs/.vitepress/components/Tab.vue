<template>
  <div v-show="isActive">
    <slot></slot>
  </div>
</template>

<script setup>
import { computed, inject } from 'vue'

defineProps({
  title: { type: String, required: true },
})
</script>
