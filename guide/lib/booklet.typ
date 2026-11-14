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

// An on-screen word, as the badge draws it: monospace on a white chip. Every
// use is also an index mark (`<gw>`): build_guide.py checks the marks against
// guide/build/data/words.json, both ways.
#let w(t) = {
  box(
    fill: white,
    stroke: 0.5pt + ink,
    inset: (x: 2pt, y: 0pt),
    outset: (y: 2pt),
    text(font: screen-font, size: 0.9em, t),
  )
  [#metadata(t) <gw>]
}

// The sprite roster (guide/build/sprites/roster.json, made by `guide sprites`).
// A missing file or key fails the compile.
#let roster = json("/guide/build/sprites/roster.json")

// A simulator screen in a bezel frame. `name` is a shot of `guide shots`.
#let screen(name, width: 46mm, caption: none) = {
  let img = image("/guide/build/screens/" + name + ".png", width: width, scaling: "pixelated")
  let framed = box(
    stroke: 2.5pt + ink,
    inset: 3pt,
    fill: rgb("#6d6a63"),
    box(stroke: 1pt + ink, img),
  )
  if caption == none {
    framed
  } else {
    box(width: width + 12pt, align(center, stack(spacing: 3pt, framed, text(size: 8pt, caption))))
  }
}

// One frame of a pet animation. `pet` is a prefix as in roster.json ("00",
// "05", ...) or "hero". `anim` is a name from roster.json's anims.
#let sprite(pet, anim, frame: 0, width: auto) = {
  let prefix = if pet == "hero" { roster.hero.prefix } else { pet }
  image(
    "/guide/build/sprites/" + prefix + "-" + anim + "-" + str(frame) + ".png",
    width: width,
    scaling: "pixelated",
  )
}

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
