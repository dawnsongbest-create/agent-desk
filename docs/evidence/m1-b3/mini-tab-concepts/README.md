# M1-B3 Mini Tab Design Spike

Status: `DESIGN_ONLY` / `NOT_IMPLEMENTED`

These concepts are isolated visual proposals. None is connected to the production Sticky state machine, persistence, drag behavior, database or default UI.

## Concept A — Bookmark Strip

- Shape: a narrow vertical paper bookmark with a clipped lower notch.
- Approximate size: 28 × 92 logical px.
- Information: Sticky color, small `3` open-Todo count and a tiny ruled-paper mark.
- Placement: rests on the right Reader edge, like a bookmark between pages.
- Expand interaction: click/tap the visible paper strip to restore Compact Sticky inward from the edge.
- Why: strongest Reader/book association and the smallest text obstruction.
- Tradeoff: the slim hit target needs careful accessibility treatment.

## Concept B — Folded Paper Corner

- Shape: a triangular folded page corner layered over the Reader canvas.
- Approximate size: 58 × 58 logical px.
- Information: paper color plus a restrained `3` count on the fold.
- Placement: top-right or bottom-right Reader corner.
- Expand interaction: click/tap the fold; Compact Sticky appears as if unfolded from that corner.
- Why: physical paper language is immediate and almost disappears into a page.
- Tradeoff: corner placement competes with future Reader controls and communicates less personal presence.

## Concept C — Tiny Sticky Tab

- Shape: a small horizontal taped Sticky sliver.
- Approximate size: 104 × 42 logical px.
- Information: `TODAY`, up to two tiny Todo ticks and a count.
- Placement: free on the Reader canvas or snapped to a safe edge.
- Expand interaction: click/tap the paper sliver to restore Compact Sticky at the same normalized position.
- Why: most clearly feels like the existing Agent Desk Sticky and offers the best recognition.
- Tradeoff: covers more Reader text than A or B.

Required decision:

`PRODUCT_OWNER_MINI_TAB_SELECTION: A / B / C / REQUESTED_REVISION`
