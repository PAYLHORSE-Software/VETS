<h1 align="center"><img src="https://github.com/paylhorse/valmar/assets/74363924/dc6fc259-1c13-4e75-9567-db63a9f97659" width=60px></img> Valmar's Eye Translation Software (VETS) <img src="https://github.com/paylhorse/valmar/assets/74363924/dc6fc259-1c13-4e75-9567-db63a9f97659" width=60px></img></h1>
<p align="center">
<strong>A utilitarian screen translation tool for Japanese games</strong>
</p>

![VETS_preview](https://github.com/paylhorse/VETS/assets/74363924/3f79d151-60be-4e78-8cbb-4873ef82f213)

## ABOUT

Current translation solutions for Japanese games are too messy for general use. VETS hopes to be a fully-featured, unintrusive, utilitarian solution for tackling that text-heavy imported JRPG or VN with confidence.

VETS uses Google Cloud Vision to spot text, ![KAKASI](http://kakasi.namazu.org/index.html.en) for romaji conversion, and DeepL machine translation.

## FEATURES

- **Unintrusive:** No overlays, drag-to-capture shenanigans or multi-window setups. Fix VETS to an area of your screen, and update translation with a keypress. Translation is organized into an ordered list of 'packets', one for each block of text.
- **Universal:** VETS works through screen capture: if it's visible on your screen, then VETS can read it for you.
- **Romaji Processing:** Optional phonetic transcription, for those learning to read!
- **Cutting-Edge OCR and Translation:** The gcloud and DeepL set up might be a little involved, but the end result really is the best of the best screen reading.

## SETUP

**VETS uses cloud-based services to function, and you'll have to set them up on your side due to API access limits :(**

This should be enough of a guide to do so, and fill out the 'CREDENTIALS' field in the VETS settings menu.

### Linux Requirements

On Linux, you need `libxcb`, `libxrandr`, and `dbus` in order for VETS to capture your screen. Odds are you already have them installed.

**Debian/Ubuntu:**

```sh
apt-get install libxcb1 libxrandr2 libdbus-1-3
```

**Alpine:**

```sh
apk add libxcb libxrandr dbus
```

**ArchLinux:**

```sh
pacman -S libxcb libxrandr dbus
```
### Google Cloud Vision

The requirement is as follows: to have a **Google Cloud Project** on your Google account, and to have the **Vision AI service** activated on that project.

https://cloud.google.com/vision/docs/setup

You can follow the guide above up to installing and running the gcloud CLI tool:

```bash
gcloud init
```

Once that's done, you're good to go! Just run the commands shown in the VETS settings menu in order to get your credentials.

> **Note:**
> The Google Cloud Token gives temporary access: it's refreshed every hour or so. Make sure to run `gcloud auth print-access-token` again if you get a warning from VETS that you need to check your credentials!

### DeepL

This is much simpler: you just need to sign up to the DeepL API through the link below. It should immediately display your personal token as soon as you're signed up.

https://www.deepl.com/pro-api?cta=header-pro-api

All done! Please feel free to leave an issue or reach out on Discord if you're having trouble with setup.

[![Static Badge](https://img.shields.io/badge/PAYLHORSE%20Discord%20Server%20-%20%233d4151?style=plastic&logo=discord&link=https%3A%2F%2Fdiscord.com%2Finvite%2FyzpZ63tJzW)](https://discord.com/invite/yzpZ63tJzW)

