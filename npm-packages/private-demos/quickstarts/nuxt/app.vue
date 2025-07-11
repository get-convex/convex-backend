<script setup lang="ts">
import { api } from "@/convex/_generated/api";

const { data: tasks } = useConvexQuery(api.tasks.get);
</script>

<template>
  <div>
    <h1>Tasks</h1>
    <ul v-if="tasks">
      <li v-for="task in tasks" :key="task._id">
        <input :id="task._id" type="checkbox" :checked="task.isCompleted" />
        <label :for="task._id">{{ task.text }}</label>
      </li>
    </ul>
  </div>
</template>