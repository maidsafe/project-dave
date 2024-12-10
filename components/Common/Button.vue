<script lang="ts" setup>
export interface IButtonProps {
  variant?: "primary" | "secondary" | "tertiary";
  size?: "small" | "medium" | "large";
}

const props = defineProps<IButtonProps>();
const { variant = "primary", size = "medium" } = props;
</script>

<template>
  <button
    class="rounded-full flex items-center justify-center gap-2 transition-all duration-300"
    :class="{
      'bg-autonomi-red-300 text-white hover:bg-autonomi-red-300/80':
        variant === 'primary',
      'bg-sky-500 text-white hover:bg-sky-500/80':
        variant === 'secondary',
      'bg-autonomi-gray-600 text-white hover:bg-autonomi-gray-600/80':
        variant === 'tertiary',
      'text-xs px-4 py-1': size === 'small',
      'text-sm px-7 py-2': size === 'medium',
      'text-lg px-[30px] py-[10px]': size === 'large',
    }"
  >
    <slot></slot>
  </button>
</template>
