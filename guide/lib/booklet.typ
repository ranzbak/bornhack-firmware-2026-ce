// Booklet look: A5 portrait, cream page, dark ink, chunky Amiga-style borders.
#let cream = rgb("#f3ead3")
#let ink = rgb("#2a2118")
#let head-font = "Press Start 2P"
#let body-font = "Libertinus Serif"
#let screen-font = "DejaVu Sans Mono"

// Chunky box used for sidebars and frames.
#let chunky(body, fill: none) = block(
  width: 100%,
  inset: 8pt,
  stroke: 2.5pt + ink,
  radius: 0pt,
  fill: fill,
  body,
)

// Pixel heart drawn from rectangles (no sprite needed).
#let pixel-heart(size: 6pt, color: rgb("#c0392b")) = {
  let rows = (
    "01100110",
    "11111111",
    "11111111",
    "11111111",
    "01111110",
    "00111100",
    "00011000",
  )
  box(width: 8 * size, height: 7 * size, {
    for (y, row) in rows.enumerate() {
      for (x, ch) in row.clusters().enumerate() {
        if ch == "1" {
          place(top + left, dx: x * size, dy: y * size, rect(width: size, height: size, fill: color, stroke: none))
        }
      }
    }
  })
}

// Show rule for the whole book.
#let booklet(body) = {
  set document(title: "BornPets Player's Guide")
  set page(
    paper: "a5",
    margin: (x: 14mm, y: 16mm),
    fill: cream,
    footer: context {
      let p = here().page()
      if p > 1 {
        align(center, text(font: head-font, size: 7pt, fill: ink, str(p)))
      }
    },
  )
  set text(font: body-font, size: 10.5pt, fill: ink, lang: "en")
  set par(justify: true, leading: 0.65em)
  show heading: it => block(
    above: 1.2em,
    below: 0.8em,
    text(font: head-font, size: if it.level == 1 { 13pt } else { 9pt }, fill: ink, it.body),
  )
  show raw: set text(font: screen-font, size: 0.9em)
  body
}

#let cover() = {
  set page(footer: none)
  align(center + horizon, {
    chunky(
      {
        v(8mm)
        text(font: head-font, size: 22pt, "BornPets")
        v(4mm)
        text(font: head-font, size: 10pt, "PLAYER'S GUIDE")
        v(8mm)
        pixel-heart(size: 9pt)
        v(8mm)
      },
      fill: white.transparentize(60%),
    )
  })
  pagebreak()
}

// Closes the book: pads with "Notes" pages so the total, back cover included,
// is a multiple of 4, then the back cover with the colophon.
// `notes-art(i)` is a hook: it returns the picture for the i-th Notes page
// (Task 5 fills it with a sprite so the picture check passes).
#let finish(notes-art: i => pixel-heart(size: 5pt)) = {
  [#metadata(none) <content-end>]
  context {
    let last = query(<content-end>).first().location().page()
    let pad = calc.rem(4 - calc.rem(last + 1, 4), 4)
    for i in range(pad) {
      pagebreak()
      heading(level: 1, outlined: false, "Notes")
      align(center, notes-art(i))
    }
  }
  pagebreak()
  set page(footer: none)
  align(center + horizon, {
    pixel-heart(size: 6pt)
    v(6mm)
    text(font: head-font, size: 7pt, [Build #sys.inputs.at("commit")])
    [#metadata(sys.inputs.at("commit")) <colophon-commit>]
  })
  context [#metadata(here().page()) <guide-end>]
}
