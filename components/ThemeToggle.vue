  
  <script setup lang="ts">
//   import { useTheme } from '../composables/useTheme'
const { isDark, toggleTheme } = useTheme()
</script>

<template>
    <button v-tooltip.bottom="'Toggle Theme'" @click="toggleTheme" class="p-1 rounded-md border-2 dark:border-white/80 border-autonomi-blue-600/50 flex items-center justify-center">
      <i v-if="isDark" class="pi pi-sun text-white" style="font-size: 1rem"></i>
      <i v-else class="pi pi-moon text-autonomi-blue-600" style="font-size: 1rem"></i>
    </button>
  </template>