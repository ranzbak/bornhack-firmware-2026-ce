#import "../lib/booklet.typ": *
#import "../lib/data.typ": *
#import "../lib/needs.typ": *
#import "../lib/care.typ": *
#chapter("ch-care")[Care actions]
#set text(size: 10pt)
#set par(leading: 0.6em, spacing: 0.9em)
#set list(spacing: 0.55em)

Everything you can do for your pet is a row in a menu. This chapter walks
through every menu and every row: what it costs, how long it takes, what it
risks and why it may say no. The last pages list every refusal and every
toast.

== How a menu works

Eight icons sit on the pet screen. The *top row* holds #w("Stats"),
#w("Hibernate"), #w("Exercise") and #w("Drink"). The *bottom row* is the care
row: #w("Feed"), #w("Heal"), #w("Play") and #w("Rest"). Pick one with Fire
and a menu pops up. Up and Down move the cursor. Fire does the row.
#w("Cancel") closes the menu, and so does CAN.

#grid(columns: (1fr, auto), column-gutter: 5mm, align: top,
  [
    A row shows more than its name:
    - *A price*, like #w("<price>H"), but only in money mode. Without money
      every row is free.
    - *A wait* in place of the price, like #w("wait <time>"), when the row
      is shut for a known time.
    - *A mark* on the right edge: #w("!"). It says the dose would go wrong.
      The title bar then reads #w("risk: <what>") for the row under the
      cursor.
    - *A grey row* is shut. The cursor still stops on it.
  ],
  screen("risk-marker", width: shot-m, caption: [Every row warns.]))

*Press a shut row and the badge tells you why.* The title bar shows the
reason for a moment. All the reasons are listed at the end of this chapter.

#pic-row((("feeding", [Fed.]), ("drinking", [Watered.]), ("playing", [Played.])))

*Actions take time.* A feed or a drink takes #v("FEED_DURATION"). The dose
lands when that time is over, and the pet shows the matching animation. While
one action runs, every other row says #w("busy"). Sleep and Wake up are
instant.

*Money.* A price is charged when the action starts, and only if the action is
accepted. A refused row costs nothing. A row you cannot pay for says
#w("can't afford"). @ch-money has the HAX story, and @ch-hard the dearer one.

*Waking a sleeper.* A care action on a sleeping pet wakes it first. If its
energy is under #v("GRUMPY_ENERGY_PCT"), it wakes #w("grumpy") and loses
#v("GRUMPY_PCT") of mood. Games, Battle and #w("Only pets") refuse a sleeper
instead: #w("asleep").

#tip[The pet knows what the menu will say. The #w("Hint") row asks it, and it
never suggests a row that would refuse. See @ch-reading.]

== Feed

#grid(columns: (1fr, auto), column-gutter: 5mm, align: top,
  [
    Five foods, in this order. The title bar shows the whole name of the row
    under the cursor: #w("Frikandel spec") is the Dutch snack
    #w("Frikandel speciaal"). In Hard mode (US) it turns into a hamburger, see
    @ch-hard.

    Every feed takes #v("FEED_DURATION"). Every food adds dirt. Salad and
    apple also give a little water, and salad a little fitness. @ch-needs has
    the full dose table.
  ],
  screen("menu-feed", width: shot-m, caption: [The Feed menu.]))

#block(above: 3mm)[
    #tbl((auto, auto, auto, auto, 1fr),
      table.header([*Food*], [*Price*], [*Full*], [*Weight*], [*Mood*]),
      [#w("Salad")], pr("Salad"), sg("Salad", "full"), [none], [none],
      [#w("Apple")], pr("Apple"), sg("Apple", "full"), sg("Apple", "weight"), [none],
      [#w("Frikandel spec")], pr("Frikandel spec"), sg("Frikandel spec", "full"), sg("Frikandel spec", "weight"), sg("Frikandel spec", "mood"),
      [#w("Pizza")], pr("Pizza"), sg("Pizza", "full"), sg("Pizza", "weight"), sg("Pizza", "mood"),
      [#w("Cake")], pr("Cake"), sg("Cake", "full"), sg("Cake", "weight"), sg("Cake", "mood"),
    )
]

#block(breakable: false)[
- *The repeat-junk rule.* Frikandel, pizza and cake are *treats*. A treat
  gives its full mood only if the last treat is more than #v("DOSE_JUNK_MIN")
  old. A treat too soon gives its mood divided by
  #num("DOSE_JUNK_REPEAT_DIV"), and the row warns #w("barely felt"). Every
  treat restarts the clock. On Ozempic a treat gives no mood at all.
- *Food coma.* Feeding never says no because the belly is full. Past the top
  of the Full band, the extra (counted up to #v("DOSE_FULL_CAP_PCT")) becomes
  weight and costs energy. The row warns #w("food coma").
- *Refused when:* #w("busy") (an action is running), #w("can't afford") and
  #w("hibernating"). That is all.
]

#joke[The pet has three words for "not enough" and one for "too much". The
menu has no word for "no more cake". It only has the warning mark.]

== Drink

#grid(columns: (1fr, auto), column-gutter: 5mm, align: top,
  [
    The Drink menu has five rows: water, cola and three alcohols. Each takes
    #v("DRINK_DURATION").

    *Water.* A glass gives up to #v("DOSE_WATER_MAX_PCT") of water, but never
    lifts the pet past #v("WATER_CAP_PCT"). Only a glass of at least
    #v("DOSE_WATER_BELLY_MIN_PCT") also fills the belly a little. Its toast is
    #w("refreshed!"), the same as cola.
  ],
  screen("refusal-water", width: shot-m, caption: [Not thirsty.]))

#block(above: 3mm)[
    #tbl((auto, auto, auto, 1fr),
      table.header([*Drink*], [*Price*], [*Water*], [*Else*]),
      [#w("Water")], pr("Water"), [up to #v("DOSE_WATER_MAX_PCT")], [Booze -#v("DOSE_WATER_BOOZE_PCT").],
      [#w("Cola")], pr("Cola"), sg("Cola", "water"), [Energy #sg("Cola", "energy"), mood #sg("Cola", "mood").],
      [#w("Beer")], pr("Beer"), sg("Beer", "water"), [Booze #sg("Beer", "booze"), mood #sg("Beer", "mood").],
      [#w("Wine")], pr("Wine"), sg("Wine", "water"), [Booze #sg("Wine", "booze"), mood #sg("Wine", "mood").],
      [#w("Whiskey")], pr("Whiskey"), [none], [Booze #sg("Whiskey", "booze"), mood #sg("Whiskey", "mood").],
    )
]

*More about water.* It takes #v("DOSE_WATER_BOOZE_PCT") off the booze, and
shortens a hangover by #v("DOSE_WATER_HANGOVER_MIN") and a groggy spell by
#v("DOSE_WATER_GROGGY_MIN").

*Alcohol.* Every alcohol also gives fun, and some energy.
Its toast reads #w("+drunk"). A drunk pet feels less of the mood, a dependent
one half of that, and a tired pet gets hit harder. @ch-conditions has the
booze rules, dependence and rehab.

#block(breakable: false)[
- *Risks.* #w("sloshing") (past the water or belly band),
  #w("one too many") (past the booze band) and #w("barely felt") (already
  drunk). Also a dry mouth (booze without water) and a relapse (alcohol in
  rehab).
- *Refused when:* water is asked of a pet above #v("WATER_REFUSE_PCT"):
  #w("not thirsty"). Also #w("busy"), #w("can't afford") and
  #w("hibernating").
]

#pic-row((("drinking", [A glass.]), ("happy", [Refreshed.])))

== Rest

#grid(columns: (1fr, auto), column-gutter: 5mm, align: top,
  [
    *Sleep.* A pet with energy under #v("DOSE_NIGHT_ENERGY_PCT") starts a
    *night*. Otherwise it takes a *nap*. A tired pet wakes by itself after
    about #night(). No pet sleeps past #v("SELF_WAKE_MIN"). The row warns when
    the night would go wrong, for example with #w("groggy") (an oversleep).
    Short nights add up to #w("worn out"). The details are in
    @ch-conditions.

    *Wake up.* Ends the sleep now. A pet with energy under
    #v("GRUMPY_ENERGY_PCT") wakes #w("grumpy"), which costs #v("GRUMPY_PCT")
    of mood. A pet that overslept wakes #w("groggy"), see @ch-conditions.
  ],
  screen("menu-rest", width: shot-m, caption: [The Rest menu.]))

#block(above: 3mm)[
    #tbl((auto, auto, auto, 1fr),
      table.header([*Row*], [*Price*], [*Takes*], [*Does*]),
      [#w("Sleep")], [free], [instant], [Puts the pet to bed.],
      [#w("Relax")], pr("Relax"), [#v("PLAY_DURATION")], [Fun #sg("Relax", "fun"), strain #sg("Relax", "strain"), energy #sg("Relax", "energy").],
      [#w("Wash")], pr("Wash"), [#v("PLAY_DURATION")], [Dirt #sg("Wash", "dirt").],
      [#w("Wake up")], [free], [instant], [Ends a sleep.],
    )
]

*Relax.* A rest for the head. It lowers fun, so a pet that was near the bottom
of the Fun band turns #w("bored"), and the row warns #w("too much relax"). It
is good for strain and is allowed on an injured pet.

*Wash.* Only for a dirty pet.

#pic-row((("sleeping", [Sleep.]), ("idle", [Relax.]), ("happy", [Clean.])))

#block(breakable: false)[
*Refused when:*
- #w("asleep"): Sleep, on a pet that is already asleep.
- #w("no need"): Wake up, on a pet that is awake.
- #w("already clean"): Wash, when Clean is over #wash-clean().
- #w("busy") and #w("hibernating"). Relax and Wash are free, so never
  #w("can't afford").
]

== Heal

#grid(columns: (1fr, auto), column-gutter: 5mm, align: top,
  [
    The Heal menu has four medicines. Each has a price in money mode, and
    each is a dose with a long memory: the row shuts for a while after it.
    @ch-conditions tells how diabetes and dependence start and end.

    *Refused when:* #w("no need") (Insulin on a pet that is not diabetic,
    Rehab on one that is not dependent), #w("wait <time>") (the cooldowns
    below), #w("can't afford"), #w("busy") and #w("hibernating").
  ],
  screen("menu-heal", width: shot-m, caption: [The Heal menu.]))

#block(above: 3mm)[
    #tbl((auto, auto, auto, 1fr),
      table.header([*Row*], [*Price*], [*Takes*], [*Does*]),
      [#w("Aspirine")], pr("Heal"), [#v("HEAL_DURATION")], [Heals an injury, or a little health.],
      [#w("Insulin")], pr("Medicate"), [#v("MEDICATE_DURATION")], [Covers a diabetic pet.],
      [#w("Ozempic")], pr("Ozempic"), [#v("OZEMPIC_DURATION")], [Weight down fast.],
      [#w("Rehab")], pr("Rehab"), [#v("REHAB_DURATION")], [Protects a dependent pet.],
    )
]

*Aspirine* works three ways. On an injured pet it takes
#v("DOSE_HEAL_INJURY_MIN") off the injury and adds #v("DOSE_MED_BOOZE_PCT")
booze. On a pet with health over #v("DOSE_HEAL_HEALTHY_PCT") it does nothing
but numb: the same booze, and the row warns #w("only numbed"). On any other
pet it gives #v("DOSE_HEAL_HEALTH_PCT") health. Either way the next dose waits
#v("DOSE_MED_CD_MIN"), shown as #w("wait <time>").

*Insulin* is only for a diabetic pet. One dose covers #v("DOSE_INSULIN_MIN").
You may give the next when #v("DOSE_INSULIN_WINDOW_MIN") or less is left, so
the row says #w("wait <time>") until then.

*Ozempic* is for any pet. It takes #v("DOSE_OZEMPIC_WEIGHT_PCT") off the
weight and #v("DOSE_OZEMPIC_FIT_PCT") off the fitness. For
#v("DOSE_OZEMPIC_MIN") the pet is *on Ozempic*, and treats give it no mood.
The next dose waits #v("DOSE_OZEMPIC_CD_MIN").

*Rehab* is only for a dependent pet. It protects for #v("DOSE_REHAB_MIN"), and
the row says #w("wait <time>") while that runs. Alcohol in rehab is a relapse:
the protection ends and mood falls by #v("DOSE_RELAPSE_MOOD_PCT").

#pic-row((("healing", [Healing.]), ("sick", [Sick.])))

== Play

#grid(columns: (1fr, auto), column-gutter: 5mm, align: top,
  [
    The Play menu has up to ten rows. #w("Only pets") shows only in money
    mode.

    - #w("Battle"): pick a friend from your list and fight. Cooldown
      #v("BATTLE_COOLDOWN"). See @ch-friends.
    - #w("Play now"): the plain play.
    - #w("Only pets"): the pet works for HAX.
    - #w("Play music"): the #w("Music") menu.
    - Five mini-games.
  ],
  screen("menu-play", width: shot-m, caption: [The Play menu.]))

*#w("Play now")* costs #price("Play") and takes #v("PLAY_DURATION"). It gives
fun #sg("Play", "fun"), and a tired pet gets it doubled
(#play-fun-tired()). It costs energy #sg("Play", "energy"), belly
#sg("Play", "full"), water #sg("Play", "water") and dirt #sg("Play", "dirt"),
and gives fitness #sg("Play", "fit"). It also cuts a groggy spell by
#v("DOSE_PLAY_GROGGY_MIN"). Refused when #w("injured").

*#w("Only pets")* is free, takes #v("PLAY_DURATION") and costs energy
#sg("OnlyPets", "energy") and mood #sg("OnlyPets", "mood"), for fun
#sg("OnlyPets", "fun"). In money mode it also pays HAX: see @ch-money. It is
never held back by the price, so a broke pet can still work its way out.
Refused when #w("asleep"), #w("injured") or #w("too tired") (energy under
#v("ENERGY_LO_PCT")).

#pic-row((("playing", [Play now.]), ("tired", [Too tired for work.])))

=== The five mini-games

Pick a game and the badge opens it. Tic Tac Toe, Black Hole and Nim ask for a
difficulty first. It changes the opponent, not the prize.

#tbl((auto, 1fr),
  table.header([*Game*], [*You win by*]),
  [#w("Tic Tac Toe")], [Three in a row. A draw counts as a draw.],
  [#w("Lights Out")], [Switching every light off. Each press flips its neighbours too.],
  [#w("Black Hole")], [The lowest score next to the hole. A tie counts as a draw.],
  [#w("Nim")], [Making the opponent take the last stick.],
  [#w("BornJeweled")], [Matching gems until the moves run out. Leave with a point on the board and it counts as a win.],
)

A *win* gives fun #mini-fun(2) and a *draw* #mini-fun(1). Both cost
#v("DOSE_MINI_ENERGY_PCT") energy. A tired pet gets the fun doubled. In money
mode a win or draw also pays #v("MINIGAME_HAX_REWARD"). The toast is
#w("nice game!"). *A loss gives nothing*: no fun, no HAX, no cooldown.

After a win or draw that game rests for #v("MINIGAME_COOLDOWN"). The row shows
#w("wait <time>"). The other four games stay open, so there is always
something to play. A sleeping pet refuses all of them: #w("asleep").

#sidebar("CASUAL", sprite-fit("hero", "happy", w: 40pt, h: 40pt), rgb("#e4efc9"))[
  Casual shortens the rest after a game to
  #v("MINIGAME_COOLDOWN", preset: casual). It also moves the tired line that
  #w("Only pets") checks to #v("ENERGY_LO_PCT", preset: casual).
]

=== Play music

#w("Play music") opens the #w("Music") menu. It is free and takes no time. Pick a
song and the buzzer plays it, and the menu closes. A song does nothing for the
pet. The list, top to bottom:

#grid(columns: (1fr, 1fr, 1fr), row-gutter: 2mm,
  [#w("Startup")], [#w("Rickroll")], [#w("Imp. March")],
  [#w("Sandstorm")], [#w("Pink Panther")], [#w("Trololo")],
  [#w("Daisy Bell")], [#w("Nokia Tune")], [#w("Samsung")],
)

#joke[#w("Imp. March") is the Imperial March, and #w("Samsung") is a phone
maker's tune. No song feeds the pet. #w("Trololo") comes closest.]

== Exercise

The Exercise icon opens a menu with one row, #w("Exercise now"). It is free
and takes #v("EXERCISE_DURATION"). It is the honest way to lose weight and
gain fitness, and the one way to tear a muscle.

#grid(columns: (1fr, auto), column-gutter: 5mm, align: top,
  [
    #tbl((auto, 1fr),
      table.header([*Need*], [*Change*]),
      [Full], sg("Exercise", "full"),
      [Water], sg("Exercise", "water"),
      [Energy], sg("Exercise", "energy"),
      [Fun], sg("Exercise", "fun"),
      [Dirt], sg("Exercise", "dirt"),
      [Weight], sg("Exercise", "weight"),
      [Booze], [-#v("DOSE_EXERCISE_BOOZE_PCT")],
      [Fitness], [+#v("DOSE_EXERCISE_FIT_PCT"), if it goes well],
    )
  ],
  gym-pic())

*Strain.* A session adds #v("DOSE_EXERCISE_STRAIN_PCT") of strain. A hungry,
thirsty or tired pet gets #v("DOSE_STRAIN_NEED_PCT") of that, an overweight
one #v("DOSE_STRAIN_HEAVY_PCT") and a worn-out one
#v("DOSE_STRAIN_DEPRIVED_PCT"). A good night brings it down to
#v("DOSE_STRAIN_RESTED_PCT") and a fit pet to #v("DOSE_STRAIN_FIT_PCT").

*Injury.* If the strain passes #v("STRAIN_INJ_PCT"), the session is a disaster.
The pet is *injured* for #v("DOSE_INJURY_MIN"), loses
#v("DOSE_INJURY_HEALTH_PCT") health and gains no fitness. The row warns
#w("torn muscle"). A lesser handicap warns #w("strained") and the session
still counts. Exercise also shortens a hangover by
#v("DOSE_EXERCISE_HANGOVER_MIN") and ends a groggy spell.

*Refused when:* #w("injured"), #w("busy") and #w("hibernating").

#tip[Check the status line first. A #w("sore") pet is close to the strain
line, one session from a torn muscle. Relax lowers strain.]

== Stats

#grid(columns: (1fr, auto), column-gutter: 5mm, align: top,
  [
    The Stats menu is about looking. Nothing here costs HAX or time, and
    nothing changes the pet. It has five rows:
    - #w("View stats"): the need bars, two pages. See @ch-needs.
    - #w("Hint"): the pet says what it wants. See @ch-reading.
    - #w("Diary"): the last #v("DIARY_LEN") causes. See @ch-reading.
    - #w("Rolled stats"): the pet's three rolled traits, as bars.
    - #w("Friends"): the pets you have met. See @ch-friends.
  ],
  screen("menu-stats", width: shot-m, caption: [The Stats menu.]))

A hibernating pet keeps every row but #w("Friends"). Any button closes a view.

== Hibernate

#grid(columns: (1fr, auto), column-gutter: 5mm, align: top,
  [The Hibernate menu has two rows: #w("Hibernate") and #w("Defrost"). The
  first freezes the pet and the second thaws it. A hibernating pet answers
  every other action with #w("hibernating"). Both rows are free and instant.
  @ch-offtime has the whole story: what freezes, what does not, and the
  alarms.],
  ice-pic())

#pagebreak(weak: true)
== Every refusal

When a row says no, it uses one of these words.

#tbl((auto, 1fr),
  table.header([*The badge says*], [*What it means*]),
  [#w("already clean")], [Wash, when Clean is over #wash-clean().],
  [#w("asleep")], [Sleep on a sleeper. Also Battle, a game or #w("Only pets") on a sleeper.],
  [#w("busy")], [Another action is still running.],
  [#w("can't afford")], [Money mode: the price is more than the balance.],
  [#w("hibernating")], [The pet is frozen. Only #w("Defrost") and the Stats views work.],
  [#w("injured")], [Play now, Exercise or #w("Only pets") on an injured pet.],
  [#w("no need")], [Wake up on an awake pet. Insulin without diabetes. Rehab without dependence.],
  [#w("not thirsty")], [Water, when hydration is over #v("WATER_REFUSE_PCT").],
  [#w("pet is gone")], [The pet has left. Rare: the badge shows no menus then.],
  [#w("still an egg")], [The egg has not hatched. Rare, for the same reason.],
  [#w("too tired")], [#w("Only pets"), when energy is under #v("ENERGY_LO_PCT").],
  [#w("wait <time>")], [A cooldown or a medicine window is running. The time is the time left.],
)

*Reading the time.* #w("wait <time>") is written as #raw("45s") under a
minute, #raw("59m") under an hour, and #raw("2h05") above that.

Which rows can show a wait?
- Aspirine: #v("DOSE_MED_CD_MIN") after a dose.
- Insulin: until the window opens.
- Ozempic: #v("DOSE_OZEMPIC_CD_MIN") after a dose.
- Rehab: while the protection runs.
- Battle and the five games: their cooldowns.

The row shows the wait in place of its price. Select the row and press Fire,
and the title bar says it in full.

#pic-row((("tired", [Too tired.]), ("sleeping", [Asleep.]), ("critical", [Hurt.])))

#pagebreak(weak: true)
== Every toast

#grid(columns: (1fr, auto), column-gutter: 5mm, align: top,
  [A *toast* is the red line in the top left corner of the pet screen. It
  stays for a couple of seconds. Most toasts say that an action was accepted.
  The others tell of a friend, a battle or a station.],
  hero-pic("happy"))

#tbl((auto, 1fr),
  table.header([*Toast*], [*When, and what it means*]),
  [#w("+full")], [A food was accepted.],
  [#w("refreshed!")], [Water or cola was accepted.],
  [#w("+drunk")], [An alcohol was accepted.],
  [#w("sleeping")], [Sleep.],
  [#w("waking up")], [Wake up, or Defrost.],
  [#w("calmer +energy")], [Relax.],
  [#w("cleaner")], [Wash.],
  [#w("+fun")], [Play now.],
  [#w("Only pets!")], [The pet went to work.],
  [#w("-weight")], [Exercise or Ozempic.],
  [#w("healing")], [Aspirine.],
  [#w("+medicated")], [Insulin.],
  [#w("rehab started")], [Rehab.],
  [#w("hibernating")], [Hibernate.],
  [#w("nice game!")], [A mini-game win or draw.],
  [#w("+happy")], [A known friend met again. See @ch-friends.],
  [#w("new friend!")], [A new friend met. See @ch-friends.],
  [#w("won a battle!")], [A Battle was won. See @ch-friends.],
  [#w("lost a battle")], [A Battle was lost. See @ch-friends.],
  [#w("Food bonus!")], [A food station was tapped. See @ch-friends.],
  [#w("Heal bonus!")], [A medic station was tapped.],
  [#w("Sleep bonus!")], [A rest station was tapped.],
)

Two more things are called a toast:
- A *reason toast*: the words of the refusal list, when you press a shut row.
- A *cause toast*: two lines, what went wrong and why, as in @ch-reading.

A station tapped too soon shows a countdown, such as #raw("wait 1:30"), in
minutes and seconds. See @ch-friends.

Two toasts exist in the game's list that nothing raises today: #w("+inspired")
and #w("Inspire bonus!"). You will not meet them.
