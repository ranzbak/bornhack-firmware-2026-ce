// Chapter 10 helpers: flags and a pixel bald eagle. Digits live here, never
// in the chapter file.
#import "booklet.typ": *

// A flag in the margin.
#let flag-pic() = pixel-flag(60pt)

// A bald eagle head from square pixels: white head, yellow beak, dark body.
#let eagle-pic(size: 5pt) = {
  pic()
  let rows = (
    "..bbbbb.....",
    ".bbwwwwb....",
    "bbwwwwwwb...",
    "bwwkwwwwbyy.",
    "bwwwwwwwyyyy",
    "bwwwwwwwwyy.",
    ".bwwwwwwbb..",
    ".bbwwwwbb...",
    "bbbbbwwbbbb.",
    "bbbbbbbbbbbb",
    ".bbbbbbbbbb.",
  )
  let cols = (b: rgb("#4a3424"), w: white, k: ink, y: rgb("#e0a82e"))
  box(width: 12 * size, height: 11 * size, {
    for (y, row) in rows.enumerate() {
      for (x, ch) in row.clusters().enumerate() {
        if ch != "." {
          place(top + left, dx: x * size, dy: y * size,
            rect(width: size, height: size, fill: cols.at(ch), stroke: none))
        }
      }
    }
  })
}

// Two flags and an eagle, as a row of margin art.
#let us-row() = grid(columns: (1fr, 1fr, 1fr), align: horizon + center,
  flag-pic(), eagle-pic(), flag-pic())
