import { createFileRoute } from '@tanstack/react-router'
import { useMutation, useSuspenseQuery } from '@tanstack/react-query'
import { convexQuery, useConvexAction } from '@convex-dev/react-query'
import { api } from '../../convex/_generated/api'

export const Route = createFileRoute('/subsequent-queries')({
  component: SubsequentSuspenseQueries,
})

function SubsequentSuspenseQueries() {
  const { mutate, isPending } = useMutation({
    mutationFn: useConvexAction(api.messages.sendGeneratedMessages),
  })
  return (
    <div>
      Without consistent SSR sometimes these differ if the page is refreshed
      while they change.
      <br />
      <SameComponent />
      {isPending ? (
        'running...'
      ) : (
        <button onClick={() => mutate({ num: 100 })}>
          insert 100 messages
        </button>
      )}
    </div>
  )
}

function SameComponent() {
  const { data: messages } = useSuspenseQuery(
    convexQuery(api.messages.count, {}),
  )
  const { data: users } = useSuspenseQuery(
    convexQuery(api.messages.countUsers, {}),
  )

  if (messages !== users) {
    throw new Error(
      `Messages and users do not match! ${messages} messages ${users} users`,
    )
  }

  return (
    <div>
      {messages} messages, {users} users
    </div>
  )
}
