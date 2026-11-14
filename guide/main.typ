#import "lib/booklet.typ": *
#import "lib/data.typ": *
#show: booklet

#cover()

= Contents

This page is a placeholder. The chapters, the appendix and the index arrive
in later tasks.

An egg on this badge hatches after #v("HATCHING_TICKS"). A hungry pet
shows the word #w("hungry") under its picture.

#align(center, grid(
  columns: 2,
  column-gutter: 6mm,
  screen("content"),
  sprite("hero", "idle", width: 46mm),
))

#align(center, pixel-heart(size: 8pt))

#finish()
