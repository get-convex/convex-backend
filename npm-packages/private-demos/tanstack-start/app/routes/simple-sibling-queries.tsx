import { createFileRoute } from '@tanstack/react-router'
import { useMutation, useSuspenseQuery } from '@tanstack/react-query'
import { api } from '../../convex/_generated/api'
import { convexQuery, useConvexAction } from '@convex-dev/react-query'

export const Route = createFileRoute('/simple-sibling-queries')({
  component: SimpleSiblingQueries,
})

function SimpleSiblingQueries() {
  const { mutate, isPending } = useMutation({
    mutationFn: useConvexAction(api.messages.sendGeneratedMessages),
  })

  return (
    <div>
      Without consistent SSR sometimes these differ if the page is refreshed
      while they change.
      <br />
      But it's hard to notice: the requests are sent at very nearly the same
      time and it's hard to accidentally join this data
      <br />
      <MessagesComponent cacheBust={1} delay={0} />
      <UsersComponent cacheBust={1} delay={0} />
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

function MessagesComponent({
  cacheBust,
  delay,
}: {
  cacheBust: number
  delay: number
}) {
  const i = slow(10000 * delay)
  const { data: messages } = useSuspenseQuery({
    ...convexQuery(api.messages.count, { cacheBust }),
  })

  return (
    <div style={{ border: 'solid 1px gray', margin: '8px' }}>
      {cacheBust}: {messages} messages
    </div>
  )
}

function UsersComponent({
  cacheBust,
  delay,
}: {
  cacheBust: number
  delay: number
}) {
  const i = slow(10000 * delay)
  const { data: users } = useSuspenseQuery(
    convexQuery(api.messages.countUsers, { cacheBust }),
  )

  return (
    <div style={{ border: 'solid 1px gray', margin: '8px' }}>
      {cacheBust}: {users} users
    </div>
  )
}

function slow(n: number) {
  let result = 0
  for (let i = 0; i < n; i++) {
    result += Math.sin(i)
  }
  return result
}
