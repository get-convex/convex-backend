import { ErrorComponent } from '@tanstack/react-router'
import type { ErrorComponentProps } from '@tanstack/react-router'

export function DefaultCatchBoundary({ error }: ErrorComponentProps) {
  console.error(error)

  return <ErrorComponent error={error} />
}
