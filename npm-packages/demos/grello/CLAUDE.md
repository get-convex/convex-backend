# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Next.js 15 project named "Grello" that uses:
- **Framework**: Next.js 15 with React 19 and TypeScript
- **Backend**: Convex for real-time database and API functions
- **Styling**: Tailwind CSS 4 with shadcn/ui components
- **State Management**: Jotai for client-side state management
- **UI Components**: Radix UI primitives with custom shadcn/ui components

## Development Commands

```bash
# Start development server
npm run dev

# Build for production  
npm run build

# Start production server
npm run start

# Run linting
npm run lint
```

## Project Architecture

### Directory Structure
- `src/app/` - Next.js App Router pages and layouts
- `src/styles/` - Contains UI components and utilities (non-standard location)
  - `src/styles/components/ui/` - shadcn/ui components
  - `src/styles/lib/` - Utility functions (cn helper, etc.)
- `convex/` - Convex backend functions and schema
- `convex/_generated/` - Auto-generated Convex types and API

### Key Configuration Details

**TypeScript Paths**: The project uses custom path aliases:
- `@/styles/*` maps to `src/styles/*`
- `@/components/*` maps to `src/components/*`

**shadcn/ui Configuration**: 
- Uses "new-york" style with RSC support
- Components are located in `@/styles/components/ui/`
- Uses Lucide React for icons
- CSS variables enabled with neutral base color

**Convex Integration**:
- Real-time database backend
- Functions defined in `convex/` directory
- Auto-generated API types in `convex/_generated/`
- Use `npx convex -h` for CLI commands

### State Management
- **Jotai**: Used for client-side state management (configured in next.config.ts transpilePackages)
- **Convex**: Handles server state with real-time subscriptions

### Styling Architecture
- **Tailwind CSS 4**: Latest version with modern features
- **CVA (Class Variance Authority)**: For component variants in UI components
- **Tailwind Merge + clsx**: Combined in `cn()` utility for conditional classes
- **Custom component structure**: UI components follow shadcn/ui patterns with Radix UI primitives

## Convex Backend Patterns

When working with Convex functions:
- Queries use `query()` from `./_generated/server`
- Mutations use `mutation()` from `./_generated/server` 
- Validate arguments with `v` from `convex/values`
- Access database via `ctx.db`
- Use `useQuery()` and `useMutation()` hooks in React components

## Important Notes

- The project uses a non-standard directory structure with styles in `src/styles/` instead of `src/components/`
- TypeScript strict mode is disabled
- React Strict Mode is enabled
- All UI components should follow the established shadcn/ui patterns with proper variant handling