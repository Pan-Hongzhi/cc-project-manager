<script setup lang="ts">
// 应用图标里的像素小人（8×7 格），用作品牌标记。size 为渲染像素宽度。
withDefaults(defineProps<{ size?: number }>(), { size: 20 });
</script>

<template>
  <svg :width="size" :height="Math.round(size * 7 / 8)" viewBox="0 0 8 7" shape-rendering="crispEdges" aria-hidden="true">
    <g fill="#E07A5F">
      <rect x="1" y="0" width="6" height="1" />
      <rect x="0" y="1" width="8" height="4" />
      <rect x="1" y="5" width="2" height="2" />
      <rect x="5" y="5" width="2" height="2" />
    </g>
    <rect x="2" y="2" width="1" height="1" fill="#1C1917" />
    <rect x="5" y="2" width="1" height="1" fill="#1C1917" />
  </svg>
</template>
