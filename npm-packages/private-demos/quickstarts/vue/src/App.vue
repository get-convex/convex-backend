<script setup lang="ts">

import { useConvexQuery } from "@convex-vue/core";
import { api } from "../convex/_generated/api";

const { data, isLoading } = useConvexQuery(api.tasks.get, {});
</script>

<template>
  <div class="wrapper">
    <ul v-if="!isLoading">
      <li v-for="todo in data">
        {{ todo.text }} {{ todo.isCompleted ? "☑" : "☐" }}
      </li>
    </ul>
    <span v-else> loading... </span>
  </div>
</template>
