<script lang="ts">
	import { useQuery } from 'convex-svelte';
	import { api } from '../convex/_generated/api.js';

	const query = useQuery(api.tasks.get, {});
</script>

{#if query.isLoading}
	Loading...
{:else if query.error}
	failed to load: {query.error.toString()}
{:else}
	<ul>
		{#each query.data as task}
			<li>
				{task.isCompleted ? '☑' : '☐'}
				<span>{task.text}</span>
				<span>assigned by {task.assigner}</span>
			</li>
		{/each}
	</ul>
{/if}
