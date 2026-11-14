#import "../lib/booklet.typ": *
#import "../lib/data.typ": *
#chapter("ch-start")[Getting started]

This is your first ten minutes. Follow the steps in order. At the end your
pet has hatched, eaten, had a drink and gone to bed. #hero is waiting.

== The buttons

#pad-card()

#block(above: 3mm)[
- *Up, Down, Left, Right* move the cursor. When there is nothing left to move
  to, Left and Right flip to the next screen of the badge.
- *Fire* is the joystick pressed in. *EXE* does the same job. Both pick and
  confirm.
- *CAN* is Cancel. It steps back or closes a menu, and the menus list it as
  #w("Cancel").
]

The badge keeps several screens in a row, and Left and Right walk along it.
The game is one of them. If you get lost, keep pushing Left or Right until
the pet shows up.

== Step by step

#step(art: screen("start", width: shot-s, caption: [The start screen.]))[
  *Find the game.* Push Left or Right until the BornPets screen shows. There
  is no pet yet, so you see the start picture. Press Fire.
]

#step(art: screen("select-pet", width: shot-s, caption: [Pick a pet.]))[
  *Choose your pet.* The screen is called #w("Choose your Pet"). Up and Down
  move the highlight. The list is whatever is installed on your badge. Press
  Fire, as #w("Fire to confirm") says. Cancel does nothing here, because a
  pet is not optional.
]

#step(art: screen("select-mode", width: shot-s, caption: [Three modes.]))[
  *Choose how money works.* Next comes #w("Play with money?"). You choose
  again for every new pet.
  - #w("With money"): the pet starts with #v("START_HAX"). Food, drinks and
    medicine have prices. See @ch-money.
  - #w("Without money"): no prices, no HAX, no worries.
  - #w("Hard mode (US)"): money is on, and some prices are higher. See
    @ch-hard. Bring a flag.
]

#step(art: screen("egg", width: shot-s, caption: [The egg, counting down.]))[
  *Wait for the egg.* It wobbles for #v("HATCHING_TICKS"). The buttons do
  nothing while it counts down, except Left and Right, which still flip the
  screen. Go and get a coffee.
]

#step(art: screen("naming", width: shot-s, caption: [Naming.]))[
  *Name your pet.* When the egg hatches, the badge opens a naming screen. A
  random Danish or Dutch name is already filled in, so you can accept it.
  Press EXE. The badge asks whether to save it. Press EXE again.

  To type your own, push the joystick towards a group of letters, then again
  to narrow it down, until you land on a letter. The screen draws the arrows.
  The CMD group holds Backspace and Clear. A name can be up to
  #v("PET_NAME_MAX") letters long.
]

#step(art: screen("hatched", width: shot-s, caption: [Hatched.]))[
  *Say hello.* Your pet is alive. The word under it is its status. A calm pet
  says #w("content"). The icons are your actions. Up and Down switch between
  the two rows of icons, and Left and Right pick one.
]

#step(art: screen("menu-feed", width: shot-s, caption: [The Feed menu.]))[
  *Feed it.* Pick the #w("Feed") icon on the bottom row and press Fire. A
  list of food opens. Press Fire on #w("Salad"). With money on, the price is
  drawn after the name, like #w("<price>H"). @ch-care says what each food
  does.

  *Give it a drink.* Pick the glass (#w("Drink")) on the top row, press Fire
  and choose #w("Water"). @ch-care has the details.
]

#step[
  *Put it to bed.* Pick the #w("Rest") icon, press Fire and choose
  #w("Sleep"). The menu also has #w("Relax") and #w("Wash"). #w("Wake up") is
  greyed out until your pet is asleep.
]

#align(center, grid(columns: 2, column-gutter: 6mm, align: top,
  screen("menu-rest", width: shot-s, caption: [The Rest menu.]),
  screen("asleep", width: shot-s, caption: [Goodnight.])))

A sleeping pet says #w("asleep"). Sleep has no timer: the pet sleeps until you
wake it, or until it wakes itself. @ch-care tells you more. You did it:
hatched, fed and asleep. Well done.

== Classic or Casual

The game has two rule sets, and this guide describes Classic. To change it,
go to the Main screen, then Bornagotchi, and step the Mode row. The change
takes effect after a reboot. @ch-hackers has the details.

== When something is wrong

#beside(
  [
    - #w("No pets installed") and #w("Check PETS.CFG"): the list of pets
      comes from a file called PETS.CFG on the badge's USB drive. It is
      missing or has no usable rows. Put it back and the pets return.
      @ch-hackers shows how.
    - #w("No sprites on flash"): the badge was flashed without its pictures.
      The game still runs, but the pet area shows this note instead of your
      pet.
  ],
  hero-pic("sick"),
)

#joke[If a pet looks at you funny, check the status word first. Then check
yourself.]
