<script lang="ts" setup>
export interface IButtonProps {
  variant?: "primary" | "secondary" | "tertiary";
  size?: "small" | "medium" | "large";
  disabled?: boolean;
  loading?: boolean;
}

const props = defineProps<IButtonProps>();
const { variant = "primary", size = "medium", disabled = false, loading = false } = props;
</script>

<template>
  <button
    class="rounded-full flex items-center justify-center gap-2 transition-all duration-300"
    :class="{
      'bg-autonomi-red-300 text-white hover:bg-autonomi-red-300/80':
        variant === 'primary' && !disabled,
      'bg-sky-500 text-white hover:bg-sky-500/80':
        variant === 'secondary' && !disabled,
      'bg-autonomi-gray-600 text-white hover:bg-autonomi-gray-600/80':
        variant === 'tertiary' && !disabled,
      'bg-gray-300 text-gray-500 cursor-not-allowed': disabled,
      'text-xs px-4 py-1': size === 'small',
      'text-sm px-7 py-2': size === 'medium',
      'text-lg px-[30px] py-[10px]': size === 'large',
    }"
    :disabled="disabled || loading"
  >
    <ProgressSpinner 
      v-if="loading" 
      style="width: 1rem; height: 1rem" 
      strokeWidth="3"
      class="mr-2"
    />
    <slot></slot>
  </button>
</template>
