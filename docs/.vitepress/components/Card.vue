<template>
  <component :is="href ? 'a' : 'div'" :href="href" class="vp-card">
    <div class="card-title">{{ title }}</div>
    <div class="card-body">
      <slot></slot>
    </div>
  </component>
</template>

<script setup>
defineProps({
  title: { type: String, required: true },
  icon: { type: String, default: '' },
  href: { type: String, default: '' },
})
</script>
