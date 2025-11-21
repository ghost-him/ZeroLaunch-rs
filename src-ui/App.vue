<template>
  <router-view />
</template>

<script setup lang="ts">
// Root App Component
</script>

<style>
/* Global styles can go here or in src-ui/styles */
html,
body {
  box-sizing: border-box;
  height: 99.85%;
  margin: 0;
  padding: 0;
  overflow: hidden; /* Prevent scrollbars on the body */
  background-color: transparent; /* Important for Tauri transparent window */
}
#app {
  width: 100vw;
  height: 100%;
}
</style>
