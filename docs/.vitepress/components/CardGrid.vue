<template>
  <div class="card-grid" :data-cols="cols">
    <slot></slot>
  </div>
</template>

<script setup>
defineProps({
  cols: {
    type: [Number, String],
    default: 2,
  },
})
</script>
