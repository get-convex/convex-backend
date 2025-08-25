<script setup lang="ts">
import { useConvexQuery } from "convex-vue";
import { api } from "../convex/_generated/api";

const { data, isPending } = useConvexQuery(api.tasks.get);
</script>

<template>
  <span v-if="isPending"> Loading... </span>
  <ul v-else>
    <li v-for="todo in data">
      {{ todo.text }} {{ todo.isCompleted ? "☑" : "☐" }}
    </li>
  </ul>
</template>
