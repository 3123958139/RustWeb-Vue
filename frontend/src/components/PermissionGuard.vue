<template>
  <div v-if="hasPermission">
    <slot />
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { useAuthStore } from '@/stores/auth';
import { Permission } from '@/types';

interface Props {
  permission?: Permission;
  permissions?: Permission[];
  requireAll?: boolean;
}

const props = withDefaults(defineProps<Props>(), {
  requireAll: false,
});

const authStore = useAuthStore();

const hasPermission = computed(() => {
  if (props.permission) {
    return authStore.hasPermission(props.permission);
  }
  
  if (props.permissions) {
    if (props.requireAll) {
      return authStore.hasAllPermissions(props.permissions);
    } else {
      return authStore.hasAnyPermission(props.permissions);
    }
  }
  
  return true;
});
</script>
