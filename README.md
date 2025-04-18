# rakuyomi

**rakuyomi** is a manga reader plugin for [KOReader](https://github.com/koreader/koreader).

<p align="center">
    <img src="docs/demo.gif" width="60%" />
    <br/>
    <em><small><a href="https://seotch.wordpress.com/ubunchu/">"Ubunchu!"</a> by Hiroshi Seo is licensed under <a href="https://creativecommons.org/licenses/by-nc/3.0/">CC BY-NC 3.0</a>.</small></em>
</p>

## Installation

Download the latest release for your device from the [releases page](https://github.com/hanatsumi/rakuyomi/releases). The following builds are available:
- **Kindle (hard floats)**: should be used on _all_ Kindles running firmware ≥ **5.16.3** (in short, if you're running KOReader's `kindlehf` build, you'll need this)
- **Kindle**: should work on older Kindle firmware versions; has been reported to work Kobo and PocketBook devices; and it might work with other ARM-based e-reader devices
- **AArch64**: should be used on some very specific devices that have troubles running the Kindle build (which targets AArch32). Known to work on the reMarkable Paper Pro.

Feel free to open an issue if your e-reader is unsupported!

Place the `rakuyomi.koplugin` folder inside KOReader's `plugins` folder.

## Configuration

In order to use the plugin, you'll need to add some sources. rakuyomi is compatible with [Aidoku](https://github.com/Aidoku/Aidoku) sources. The easiest way to obtain sources is to configure _source lists_ and download the sources through the app.

You'll most likely find links with the format `https://aidoku.app/add-source-list/?url=<source list URL>`. Grab the source list URL from these links and make sure it ends with `/index.min.json`. If it doesn't, add it yourself.

Create a `rakuyomi/settings.json` file inside KOReader's home directory, with the following contents:

```json
{
    "$schema": "https://github.com/hanatsumi/rakuyomi/releases/latest/download/settings.schema.json",
    "source_lists": [
        "https://samplewebsite.com/sources/index.min.json",
        "<your source list URL>"
    ],
    "languages": ["en", "<your preferred language here>"]
}
```

After configuring a source list, sources can be installed through the plugin, by going to the main menu and going to "Manage sources".

For a better manga reading experience, you might also look into KOReader's [reader recommended settings](./docs/src/reader-recommended-settings/index.md).

## Usage

In KOReader's file manager, go to the "Search" menu and tap on the "Rakuyomi" entry to open the library view. Tap the menu button on the top left to open the main menu.
