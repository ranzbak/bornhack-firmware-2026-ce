#import "../lib/booklet.typ": *
#import "../lib/data.typ": *
#chapter("ch-reading")[Reading your pet]

Your pet cannot talk. It has a status line, a buzzer, a speech bubble and a
diary instead. This chapter teaches you to read all four.

== The screen

#grid(columns: (auto, 1fr), column-gutter: 5mm, align: top,
  annotated("content", content-marks),
  [
    - *A* is the pet area. Your pet lives here and moves about.
    - *B* is the status line. It says how your pet is doing.
    - *C* is the toast corner. A short note shows here after you do
      something, and when something goes wrong.
    - *D* is your HAX balance. It only shows in the money modes. Without
      money, the spot stays empty.
    - *E* and *F* are the two rows of icons. The top row holds the info,
      hibernate, exercise and drink icons. The bottom row is the care row.
      @ch-start showed you how to use them.
  ])

#block(above: 3mm)[A hibernating pet is frozen, so its status line is empty.
The battery meter sits in the top corner too, above the balance.]

#tip[Toasts are polite. They go away by themselves. The status line stays
until the problem is fixed.]

== The status line

The line under the pet is the most important thing on the screen. It has
one or two words. Here is how the badge picks them.

#grid(columns: (auto, 1fr), column-gutter: 5mm, align: top,
  screen("two-words", width: shot-m, caption: [Two words.]),
  [
    Every problem pulls the pet's mood down. The badge ranks the pulls and
    names the biggest one. A problem that pulls only a little is not named
    at all. If nothing is named, you get an *info word* instead.

    The *first word* is, in this order:
    + a *danger word*, if there is one;
    + else the biggest problem.

    Then come the *prompts*, #w("no insulin") and #w("needs rehab"). They
    say that a lasting condition needs a drug. A prompt takes the second
    place before another problem can.

    The *second word* is the next biggest problem. It shows only when its
    pull is more than #v("SECOND_WORD_UNDER_PCT"). There is never a third
    word.
  ])

If both words are too wide for one line, the second word drops below the
first.

=== Red means danger

A pet in danger gets a *red* status line. The danger words are checked in a
fixed order, and the first one that fits wins:

+ #w("starving"): fullness under #v("NEED_DANGER_LO_PCT").
+ #w("parched"): hydration under #v("NEED_DANGER_LO_PCT").
+ #w("wasted"): booze over #v("BOOZE_DANGER_PCT").
+ #w("filthy"): dirt over #v("DIRT_DANGER_PCT").
+ #w("poorly"): health under #v("HEALTH_DANGER_PCT").

#grid(columns: (1fr, 1fr), column-gutter: 4mm, align: top + center,
  screen("starving", width: shot-m, caption: [Danger: red.]),
  screen("wasted", width: shot-m, caption: [Booze over the line.]))

The whole line turns red, second word and all. The danger word always comes
first.

=== The info words

When nothing is wrong, the line shows one of five words. The badge tries
them in this order:

#grid(columns: (1fr, auto), column-gutter: 4mm, align: top,
  [
    + #w("asleep"): the pet is in bed.
    + #w("sore"): strain is over #v("SORE_PCT"), and no injury says it
      already.
    + #w("tipsy"): booze is at least #v("TIPSY_PCT").
    + #w("well rested"): the pet slept well and the bonus is still on.
    + #w("content"): all is well.
  ],
  screen("asleep", width: shot-m, caption: [Asleep.]))

=== Every status word

#let row(word, means, see) = (w(word), means, see)
#word-table(see: true,
  table.header([*Word*], [*What it says*], [*Read on*]),
  ..row("peckish", [Fullness is a bit low.], [@ch-needs]),
  ..row("hungry", [Fullness is under #v("HUNGRY_PCT").], [@ch-needs]),
  ..row("starving", [Fullness is in the danger zone. Red.], [@ch-needs]),
  ..row("stuffed", [Fullness is too high. Let it digest.], [@ch-needs]),
  ..row("thirsty", [Hydration is low.], [@ch-needs]),
  ..row("parched", [Hydration is in the danger zone. Red.], [@ch-needs]),
  ..row("waterlogged", [Too much water. No more drinks.], [@ch-needs]),
  ..row("sleepy", [Energy is low, and the pet is awake.], [@ch-needs]),
  ..row("exhausted", [Energy is under #v("EXHAUSTED_PCT").], [@ch-needs]),
  ..row("bored", [Fun is low, and the pet is awake.], [@ch-needs]),
  ..row("wired", [Fun is too high. The pet is overexcited.], [@ch-needs]),
  ..row("tipsy", [A little booze. An info word.], [@ch-needs]),
  ..row("sloppy", [Booze is above its comfort band.], [@ch-needs]),
  ..row("wasted", [Booze is in the danger zone. Red.], [@ch-needs]),
  ..row("grubby", [Dirt is above its comfort band.], [@ch-needs]),
  ..row("filthy", [Dirt is in the danger zone. Red.], [@ch-needs]),
  ..row("overweight", [Weight is too high.], [@ch-needs]),
  ..row("underweight", [Weight is too low.], [@ch-needs]),
  ..row("unfit", [Fitness is too low.], [@ch-needs]),
  ..row("hung over", [A hangover is running.], [@ch-conditions]),
  ..row("sore", [Strain is high. An info word.], [@ch-needs]),
  ..row("injured", [An injury is running.], [@ch-conditions]),
  ..row("groggy", [A groggy spell is running.], [@ch-conditions]),
  ..row("poorly", [Health is low. Red in the danger zone.], [@ch-mood]),
  ..row("no insulin", [A diabetic pet needs its medicine.], [@ch-conditions]),
  ..row("needs rehab", [A dependent pet needs rehab.], [@ch-conditions]),
  ..row("asleep", [In bed. An info word.], [@ch-care]),
  ..row("well rested", [A good sleep bonus. An info word.], [@ch-care]),
  ..row("content", [Nothing to name. All is well.], [@ch-mood]),
)

== The buzz

The badge has a buzzer, and it speaks up less than you might fear. It buzzes
only when things get *worse*, and only for these words. Think of a ladder
with four steps: calm, warning, danger and leaving.

- *Buzzes:* #w("hungry"), #w("starving"), #w("thirsty"), #w("parched"),
  #w("exhausted"), #w("wasted"), #w("filthy"), #w("hung over"),
  #w("injured"), #w("groggy") and #w("poorly"). Any danger word buzzes.
- *Quiet:* every other word, such as #w("peckish"), #w("sleepy"),
  #w("bored") or the prompts.

The buzzer sounds when the pet steps *up* the ladder: from calm to warning,
from warning to danger, and from danger to leaving. It does not sound for
getting better, and it does not repeat while the pet stays on one step. It
stays silent right after the badge boots, so a pet that was already hungry
does not wake the room. A pet that has just gone gets a short, silly tune of
its own.

The Mute row in the badge's main menu turns the game's buzzer off.

#joke[The pet does not buzz for #w("peckish"). It is only peckish. It will
survive. It always says that.]

== The idle frame

The e-paper screen only uses power when the picture changes. So the badge
does not animate an idle pet. It draws *one frame every*
#v("IDLE_FRAME_SECS"). A calm pet shows one frame, waits, and shows the next.
That spares the battery and the panel.

*Actions animate at full speed.* When you feed, drink, heal, play, exercise,
medicate, take Ozempic or go to rehab, the pet moves quickly for those few
seconds. The egg animates quickly too.

The screen also redraws when something happens, such as a toast, a new status
word or a menu. So if your pet looks frozen, it is not. It is napping
between frames.

#grid(columns: (1fr, 1fr), column-gutter: 3mm, align: bottom + center,
  hero-pic("idle"),
  hero-pic("happy"))

== The Hint row

Do not know what your pet wants? Ask it. Pick the Stats icon, then the
#w("Hint") row. A speech bubble opens with two lines. It is titled with the
pet's name: #w("<name> says"). A pet with no name says
#w("Your pet says").

#grid(columns: (auto, 1fr), column-gutter: 5mm, align: top,
  screen("hint", width: shot-m, caption: [A hint.]),
  [
    The *first line* is the pet naming its state. For most words it reads
    #w("I'm <word>!"), for example "I'm starving!". The word is the first
    word of the status line. The special ones are:
    - #w("I'm diabetic!") for #w("no insulin"), and #w("I'm dependent!") for
      #w("needs rehab").
    - #w("Zzz...") when asleep, #w("I feel great!") when well rested, and
      #w("I'm happy!") when content.
    - A tired-out pet with nothing worse to say reads #w("I'm <word>!") with
      the word "worn out".
    - #w("*wobble*") for an egg, #w("...") for a pet that has left, and
      #w("Brrr, frozen!") for a hibernating one.
  ])

The *second line* is what the pet wants you to do:

#word-table(see: false,
  table.header([*The pet says*], [*Do this*]),
  [#w("Please feed me.")], [Feed it.],
  [#w("Water, please.")], [Give it water.],
  [#w("Put me to bed.")], [Sleep.],
  [#w("Play with me!")], [Play with it.],
  [#w("Play a game!")], [Play a mini-game.],
  [#w("Let me relax.")], [Relax.],
  [#w("Wash me, please.")], [Wash.],
  [#w("Let's work out!")], [Exercise.],
  [#w("I need medicine.")], [Heal, for an injury or poor health.],
  [#w("I need insulin.")], [Give insulin, from the Heal menu.],
  [#w("I need rehab.")], [Rehab, from the Heal menu.],
  [#w("Defrost me!")], [Defrost.],
)

Some words have nothing to ask for. The pet then says a fixed line:
#w("Let me digest.") when stuffed, #w("No more drinks.") when waterlogged,
#w("Hic! Thanks!") when tipsy, #w("Let me sleep.") when asleep and
#w("Thanks!") when well rested or content. An egg says #w("I'm hatching!")
and a pet that has left says #w("Your pet left.").

=== The pet never asks for the impossible

The pet only asks for something the menu would accept. If there are
several ways out, it asks for a clean one first, and it picks one with a
catch only when none is clean. When the menu would refuse every way, the
pet tells you why: #w("but <reason>"). The reasons are the ones the menu
shows, such as #w("too tired") or #w("can't afford"). @ch-care lists them
all.

#tip[The Hint row costs nothing, and the pet never lies. When in doubt,
ask.]

== The Diary

Something went wrong, and you missed it? Pick the Stats icon, then the
#w("Diary") row. The diary keeps the last #v("DIARY_LEN") causes, newest
first.

#grid(columns: (auto, 1fr), column-gutter: 5mm, align: top,
  stack(spacing: gap-s,
    screen("cause-toast", width: shot-m, caption: [The toast.]),
    screen("diary", width: shot-m, caption: [The diary.])),
  [
    When something goes wrong, a *cause toast* shows in the toast corner. It
    has two lines: what went wrong, and why. It is silent. It is also
    written in the diary.

    Each diary entry has two lines, like the toast. The first starts with
    the pet's *age* at the time: #w("now"), #w("<n>m") for minutes,
    #w("<n>h") for hours and #w("<n>d") for days. The second line says why.

    - It holds #v("DIARY_LEN") entries. The oldest drops off.
    - Three entries fit a page. With more, there are two pages. The title
      reads #w("Diary 1/2") and #w("Diary 2/2"). Left and Right flip pages.
    - Any other button closes it. It always opens at the first page.
    - With no entries it says #w("Nothing yet"). A short diary shows only
      the title #w("Diary").
    - The diary is *not saved*. Reboot the badge, or hatch a new pet, and it
      starts empty.
    - The ages tick on by themselves while it is open.
  ])

If a full-screen view is open when something goes wrong, the toast stays
away. The cause still lands in the diary.

#joke[Never mind. Nobody reads their own diary after a bad night. Your pet
does not, either.]
