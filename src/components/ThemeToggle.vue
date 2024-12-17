<template>
  <button @click="toggleTheme" class="p-2 rounded-md border dark:border-white border-black">
    {{ isDark ? '🌙' : '☀️' }}
  </button>
</template>

<script setup lang="ts">
import { useTheme } from '../composables/useTheme'

const { isDark, toggleTheme } = useTheme()
</script>
