<template>
  <div class="vp-tabs">
    <div class="vp-tabs-nav">
      <button
        v-for="(tab, index) in tabs"
        :key="index"
        :class="{ active: activeTab === index }"
        @click="activeTab = index"
      >
        {{ tab }}
      </button>
    </div>
    <div class="vp-tab-panel">
      <slot :name="'tab-' + activeTab"></slot>
    </div>
  </div>
</template>

<script setup>
import { ref, useSlots, computed } from 'vue'

const props = defineProps({
  labels: {
    type: Array,
    default: () => [],
  },
})

const activeTab = ref(0)
const tabs = computed(() => props.labels)
</script>
