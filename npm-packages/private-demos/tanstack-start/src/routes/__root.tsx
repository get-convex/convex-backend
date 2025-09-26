import { QueryClient } from '@tanstack/react-query'
import {
  createRootRouteWithContext,
  HeadContent,
  Scripts,
  Outlet,
} from '@tanstack/react-router'
import * as React from 'react'

export const Route = createRootRouteWithContext<{ queryClient: QueryClient }>()(
  {
    head: () => ({
      meta: [
        {
          charSet: 'utf-8',
        },
        {
          name: 'viewport',
          content: 'width=device-width, initial-scale=1',
        },
        {
          title: 'TanStack Start Starter',
        },
      ],
    }),
    component: RootComponent,
  },
)

function RootComponent() {
  return (
    <RootDocument>
      <Outlet />
    </RootDocument>
  )
}

function RootDocument({ children }: { children: React.ReactNode }) {
  return (
    <html>
      <head>
        <HeadContent />
      </head>
      <body>
        <div>{children}</div>
        <Scripts />
      </body>
    </html>
  )
}
