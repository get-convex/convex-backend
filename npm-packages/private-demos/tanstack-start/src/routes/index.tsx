import { Link, createFileRoute } from '@tanstack/react-router'

export const Route = createFileRoute('/')({
  component: Home,
})

function Home() {
  return (
    <ul>
      <li>
        <Link to="/simple-sibling-queries">
          Simple sibling queries with no router
        </Link>
      </li>
      <li>
        <Link to="/subsequent-queries">
          One component with two useSuspenseQuery calls
        </Link>
      </li>
    </ul>
  )
}
