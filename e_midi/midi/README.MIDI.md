# MIDI Sound Effects Collection

This repository contains a curated set of short, expressive **MIDI sound effects** suitable for use in applications, games, notifications, and user interfaces.

## 🎵 Sound Index

| File Name                | Description                                                            |
| ------------------------ | ---------------------------------------------------------------------- |
| `success.mid`            | Upward arpeggio (C major) — classic success                            |
| `success_2.mid`          | Triumphant rising tones — enhanced success                             |
| `error.mid`              | Downward arpeggio — classic error/failure                              |
| `error_2.mid`            | Low dissonant tones — stronger error feel                              |
| `notice.mid`             | Neutral two-note chime — general info                                  |
| `confirm.mid`            | Gentle confirmation — success-lite                                     |
| `alert.mid`              | High-pitched alert ping                                                |
| `warning.mid`            | Low-high tone warning                                                  |
| `panic.mid`              | Fast dissonant alarm tones                                             |
| `panic_2.mid`            | Rapid alternation — dramatic panic                                     |
| `coin.mid`               | Game-style coin pickup jingle                                          |
| `powerup.mid`            | Classic power-up scale                                                 |
| `0-song-entertainer.mid` | Scott Joplin's "The Entertainer" (public domain, via Mutopia Project)  |
| `1-song-maple.mid`       | Scott Joplin's "Maple Leaf Rag" (public domain, via Mutopia Project)   |
| `2-song-winners.mid`     | Scott Joplin's "The Easy Winners" (public domain, via Mutopia Project) |

Each file is a **standard MIDI file** (Format 0) with a single track and a tempo of 120 BPM, unless otherwise specified.

---

## 📜 Third-Party MIDI File Documentation

### `0-song-entertainer.mid`

Scott Joplin's "The Entertainer" (c. 1902) is included as `0-song-entertainer.mid` in this collection. This MIDI file was sourced from the [Mutopia Project](https://www.mutopiaproject.org/cgibin/make-table.cgi?searchingfor=entertainer), which provides public domain sheet music and MIDI files. According to the [Mutopia Project licensing page](https://www.mutopiaproject.org/legal.html#publicdomain), this file is dedicated to the public domain:

> The contributor of this music has dedicated their contribution into the public domain. You can do what you like with this music - print it out, sell it, change it, distribute it, record it, and perform it, etc.

No attribution is required. See the [Mutopia Project legal page](https://www.mutopiaproject.org/legal.html#publicdomain) for details.

---

### `1-song-maple.mid`

Scott Joplin's "Maple Leaf Rag" (c. 1899) is included as `1-song-maple.mid` in this collection. This MIDI file was sourced from the [Mutopia Project](https://www.mutopiaproject.org/cgibin/make-table.cgi?searchingfor=maple+leaf+rag), which provides public domain sheet music and MIDI files. According to the [Mutopia Project licensing page](https://www.mutopiaproject.org/legal.html#publicdomain), this file is dedicated to the public domain:

> The contributor of this music has dedicated their contribution into the public domain. You can do what you like with this music - print it out, sell it, change it, distribute it, record it, and perform it, etc.

No attribution is required. See the [Mutopia Project legal page](https://www.mutopiaproject.org/legal.html#publicdomain) for details.

---

### `2-song-winners.mid`

Scott Joplin's "The Easy Winners" (c. 1901) is included as `2-song-winners.mid` in this collection. This MIDI file was sourced from the [Mutopia Project](https://www.mutopiaproject.org/cgibin/make-table.cgi?searchingfor=easy+winners), which provides public domain sheet music and MIDI files. According to the [Mutopia Project licensing page](https://www.mutopiaproject.org/legal.html#publicdomain), this file is dedicated to the public domain:

> The contributor of this music has dedicated their contribution into the public domain. You can do what you like with this music - print it out, sell it, change it, distribute it, record it, and perform it, etc.

No attribution is required. See the [Mutopia Project legal page](https://www.mutopiaproject.org/legal.html#publicdomain) for details.

---

## 🛠️ Usage

You can use these `.mid` files in:

* Game engines (Unity, Godot, etc.)
* Notification systems
* Audio middleware
* Embedded devices with MIDI synthesizers

Use freely. No attribution required.

---

## 🆓 Public Domain Dedication

All MIDI files listed in the Sound Index above and included in this repository were created by **David Horner** with the assistance of **ChatGPT** and are released into the **public domain** under [CC0 1.0 Universal](https://creativecommons.org/publicdomain/zero/1.0/).

> You may copy, modify, distribute, and perform the work, even for commercial purposes, all without asking permission.

This dedication specifically applies to the `.mid` files listed in this document.

---

## 💡 LLM System Prompt Template

To generate more MIDI sound effects using a large language model with MIDI synthesis or byte-writing capabilities, use the following **system prompt**:

```
You are a MIDI sound designer. Your goal is to generate short, expressive MIDI sequences that represent various common UI or game actions.

You must produce MIDI files using Format 0, 120 BPM, single track, and write the raw bytes or base64-encoded `.mid` file for each sound.

Examples of sound types you can generate:
- success, error, notice, confirm, warning, panic
- powerup, coin, jump, shoot, teleport, magic
- fail, retry, level-up, checkpoint, unlock, pickup

Ensure each sound is 0.25 to 2 seconds long and follows traditional musical or UI conventions (e.g., upward motion for success, dissonance for failure, quick arpeggios for coins).

Provide the file content and a short description for each sound.
```

---

## 📁 License Summary

This folder and the midi files listed above are released under:

```
CC0 1.0 Universal (CC0 1.0) Public Domain Dedication
```

You can copy, modify, and use the listed files without restriction.

---
