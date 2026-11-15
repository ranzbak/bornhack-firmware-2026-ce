# NFC — User Guide

The back of the badge has an NFC antenna. Tap a phone or a station reader to the badge to interact.

## Two things happen on a tap

### 1. Phone reads your broadcast data

Any standard NFC reader (Android default, iOS) sees whatever you've set as your broadcast profile — by default a `https://badge.team/docs/badges/bornhack-2026/` URL, but you can replace it with your own vanity URL or a vCard (see [Set your own broadcast data](#set-your-own-broadcast-data)). Tapping with the OS reader just reads it. Harmless.

### 2. BadgeCtl runs a station command

A phone running the **BadgeCtl** companion app (loaded with the matching event private key) can send signed commands when held to the badge. These are typically used at event "stations" where you can boost your pet's stats.

The magic places:

| Place         | What it does for your pet                              |
| ------------- | ------------------------------------------------------ |
| Food stall    | more food and water                                    |
| Pharmacy      | health up, booze down, clears hangover, injury, groggy |
| Nap tent      | full energy, clears groggy and deprived                |
| Spa           | cleaner, calmer, less strain                           |
| Bar recovery  | the morning after: less booze, more water              |
| Water point   | more water                                             |
| Gym           | a little fitter (long wait before the next visit)      |
| Clinic (rare) | clears dependence and diabetes (very long wait)        |

The badge never pushes a need into a danger zone, so a station cannot
overfeed your pet. Every fix also cheers it up a little.

The badge answers every tap with a toast:

| Toast                                  | Meaning                                    |
| -------------------------------------- | ------------------------------------------ |
| `Magic fix!` / `at <station>`          | it worked                                  |
| `Magic fix!` / `thanks!`               | it worked, and your demo pet says thanks   |
| `Covered by` / `magic, not insurance`  | it worked (hard mode, no bill)             |
| `Too soon!` / `again in <time>`        | this station is on cooldown (5 minutes or more) |
| `pet is gone`                          | your pet has left; start a new egg         |
| `Unknown magic` / `badge can't read it`| the badge does not know that message       |

Station commands need an **active game**: pick a pet first (the egg
countdown already counts). With no game at all there is no toast: the
tap brings the start screen forward.

## What you need to do

Nothing on the badge. The badge is always ready. Just hold the back of the badge close to the reader for a moment.

The reader side needs:

- The **BadgeCtl** app installed
- The matching Ed25519 private key bundled in (BornHack staff have it for stations)

Third-party / random NFC reader apps cannot issue these commands — they don't have the key. They just see the public URL.

## Set your own broadcast data

You can replace the default `badge.team` URL with **anything you want the
badge to hand out** — a vanity URL, a vCard business card, a Wi-Fi record,
whatever. Use any NFC-writer app on your phone (e.g. "NFC Tools") and
write to the badge. The rule is simple: **anything you write sticks,
except a `token:` (that one is transient — it doesn't become your
persisted broadcast profile).**

- **Vanity URL** — write a **URL / URI** record (e.g. `annejan.com`). The
  badge starts serving it. A **Text** record `set:https://your.link` also
  works for writers that only emit text.
- **vCard** — write a **Contact / vCard** record. Other phones tapping you
  get your contact card.
- **Wi-Fi**, or any other record — served verbatim, same deal.

Your choice is saved and survives a reboot. Keep it short — the badge
caps records at ~127 bytes (fine for a URL or a compact vCard). Long
URLs sent via the `set:` text form are additionally clamped to ~118
characters after the scheme — for anything longer, write a plain URI
record instead, or use a link shortener.

There is no NFC "erase" back to the factory default: an empty write (or
a writer app's "format tag") doesn't restore the `badge.team` URL — it
just leaves your current profile in place (or stores the empty record).
To change what you broadcast, simply write the new record over it.

When someone taps a **token** onto your badge, that write shows for about
**10 seconds** and then your badge goes back to broadcasting your own data
— a pushed token can't overwrite your profile.

Note: setting this is **unauthenticated** — anyone who can physically
tap your badge with a writer app can change it. It's your badge in your
pocket; treat physical access accordingly.

## If you want to run your own station

You need to rebuild the badge firmware with your **own** Ed25519 public key, then sign the matching commands with your private key in your reader app. See [NFC_README.md](NFC_README.md) for the protocol spec and signing recipe.
