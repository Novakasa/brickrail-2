# Layout

The layout is the model of the physical track plan that the editor builds and
the [train simulation](train-simulation.md) runs on. This file describes its
geometry and the graphs derived from it.

## Grid

The layout lives on a square grid. A **cell** has integer coordinates and a
**layer**; layers let tracks cross or run above one another without touching.
Each cell has four edges, named by the **cardinal** directions north, east,
south and west.

## Tracks

A **track** (code: `TrackID`) occupies exactly one cell and joins two of its
four edges, so it has one of six **orientations**: two straights (north–south,
east–west) and four curves (one per pair of adjacent edges). A track's ends sit
at the midpoints of the two edges it joins. A track is identified by its cell
and orientation; its textual name has the form `x,y,layer|NS`.

A **directed track** is a track together with the end it is heading toward.
Every track has exactly two directed variants, each the opposite of the other.

## Connections

A **connection** joins two tracks end to end. A connection is **continuous**
when the two tracks sit in neighbouring cells and both face the edge they
share: a train rolls from one onto the other. Any other connection is a
**portal**: it joins two tracks that are not spatially adjacent, which is how
a route continues across layers or across a gap the grid cannot show. The
simulation treats both kinds alike.

Connections are identified by their two directed tracks, each pointing toward
the other, with the pair normalized so that one connection has one identity.
Together they form the **connection graph**, an undirected graph over tracks,
and every pathfinding structure is derived from it.

## Blocks on the grid

A block's section is an ordered list of directed tracks; the directed section
for travel **against** the stored order is the reverse list with every track
flipped. A block is identified by the normalized pair of its two endpoint
tracks. Everything else about blocks, markers and sections is in the train
simulation design.

## Logical tracks and the logical graph

A **logical track** is a directed track together with a facing: the full
state of a train's sensor on that track. The **logical graph** is the directed
graph over logical tracks on which pathfinding and leg construction operate.
It is built from the connection graph and the blocks by two rules:

- **Connections preserve facing.** Every connection yields edges for both
  facings and both travel directions: from a directed track facing the
  shared edge to the opposite of the directed track on the far side, with the
  same facing. Four edges per connection.
- **Reversal happens only inside blocks.** At each of a block's two endpoint
  tracks, a logical track is joined both ways to its reversed variant, the
  opposite directed track with the opposite facing. A train therefore changes
  travel direction only by stopping in a block and leaving the way it came,
  and doing so flips its facing.

The logical graph depends only on the layout and changes only when the layout
does.
