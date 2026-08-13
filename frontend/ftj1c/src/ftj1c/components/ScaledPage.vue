<!--
  ScaledPage.vue — 非仪表盘页面通用缩放容器（ftj1c 模块）

  与主仪表盘（Monitor.vue）共用 useWindowScale 的 1920×1080 设计尺寸 CSS scale 缩放，
  保证所有页面与主仪表盘的大小/缩放比例完全一致。内容通过默认插槽放入。

  用法：
  <ScaledPage>
    <div class="ftj1c-root">
      <div class="fm-page">…</div>
    </div>
  </ScaledPage>
-->
<template>
  <div ref="rootRef" class="screen-root">
    <div
        :style="{
        width: DESIGN_W + 'px',
        height: DESIGN_H + 'px',
        transform: `scale(${scale.x}, ${scale.y})`,
      }"
        class="scaled-stage"
    >
      <slot/>
    </div>
  </div>
</template>

<script lang="ts" setup>
import {useWindowScale} from '@/ftj1c/composables/useWindowScale'

const {scale, rootRef, DESIGN_W, DESIGN_H} = useWindowScale()
</script>

<style scoped>
.screen-root {
  width: 100vw;
  height: 100vh;
  overflow: hidden;
  display: flex;
  justify-content: center;
  align-items: center;
  background: var(--bg-page);
  transition: background 0.3s;
}

.scaled-stage {
  transform-origin: center center;
  overflow: hidden;
  flex-shrink: 0;
}
</style>
