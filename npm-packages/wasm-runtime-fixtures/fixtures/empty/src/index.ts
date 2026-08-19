// Size-measurement floor fixture: the smallest possible user bundle.
// Used by the W4 artifact-size comparison to separate engine floor from
// per-fixture growth. Deliberately touches no host surface.
export function noop(): null {
  return null;
}
