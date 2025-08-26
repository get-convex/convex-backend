<script setup lang="ts">
import { api } from "../convex/_generated/api";
const { data: tasks } = useConvexQuery(api.tasks.get);
</script>

<template>
  <div>
    <h1>Tasks</h1>
    <ul>
      <li v-for="task in tasks" :key="task._id">
        <span>{{ task.text }}</span>
      </li>
    </ul>
  </div>
</template>