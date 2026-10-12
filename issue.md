# Issue #409: geo: deterministic hierarchical spatial cell index for CRS84 points (assignment, parent/child, covering)

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

Radius and box search over point literals needs a hierarchical cell index: each
point is keyed by the cell that contains it at a fixed resolution, and a query
covers its region with a set of cells and refines by exact distance. `purrdf-geo`
has no cell index, so a consumer doing this keeps a third-party hexagonal-grid
library beside purrdf, and the cell ids it stores are only as stable as that
library's version.

## Ask

A deterministic hierarchical spatial cell index in `purrdf-geo` for CRS84 points:

- **Cell assignment:** point (longitude, latitude) and resolution to a cell id, a
  pure function of its inputs, identical on every host and target (no floating
  point that a platform `libm` can move; exact or fixed-point arithmetic, or a
  declared, tested rounding contract).
- **Parent / children** across resolutions, so a cell can be coarsened (for
  example to bound disclosure precision) or refined.
- **Covering:** the set of cells at a resolution that covers a disk of a given
  radius in metres (paired with the geodesic distance requested separately) and a
  bounding box, with a declared, finite bound on the cover size and a typed refusal
  past it rather than a silently truncated cover.
- **Range checks:** latitude outside [-90, 90] or longitude outside [-180, 180]
  refused with a typed error; never assigned a plausible cell.
- **Identity:** the grid and its parameters named in a versioned profile, so a
  change re-mints the identity of every index built on it.

H3 compatibility (same cell ids as the H3 reference) would be welcome but is not
required; determinism and a stable, versioned definition are.

## Acceptance

- Conformance vectors for assignment, parent/child, and covering, including the
  poles, the antimeridian, and resolution extremes.
- The cover of a radius contains every cell that holds a point within the radius
  (no false negatives), checked exhaustively on small radii.
- Out-of-range coordinates refuse.


## Comments (0)

