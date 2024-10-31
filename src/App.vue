<script setup lang="ts">
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { Search } from '@element-plus/icons-vue'

const greetMsg = ref("");
const name = ref("");
const input3 = ref('');
const select = ref('');


async function greet() {
  // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
  greetMsg.value = await invoke("greet", { name: name.value });
}
</script>

<template>
  <main>
    <h1>Welcome to Tauri + Vue</h1>
    <div>
    <el-input
      v-model="input3"
      style="max-width: 600px"
      placeholder="Please input"
      class="input-with-select"
    >
      <template #prepend>
        <el-select v-model="select" placeholder="Select" style="width: 115px">
          <el-option label="Restaurant" value="1" />
          <el-option label="Order No." value="2" />
          <el-option label="Tel" value="3" />
        </el-select>
      </template>
      <template #append>
        <el-button :icon="Search" />
      </template>
    </el-input>
    </div>
  </main>

</template>

<style scoped>
.logo.vite:hover {
  filter: drop-shadow(0 0 2em #747bff);
}

.logo.vue:hover {
  filter: drop-shadow(0 0 2em #249b73);
}

</style>
<style>

h1 {
  text-align: center;
}


.input-with-select .el-input-group__prepend {
  background-color: var(--el-fill-color-blank);
}

</style>