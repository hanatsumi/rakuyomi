# rakuyomi

**rakuyomi** is a manga reader plugin for [KOReader](https://github.com/koreader/koreader).

<p align="center">
    <img src="docs/demo.gif" width="60%" />
    <br/>
    <em><small><a href="https://seotch.wordpress.com/ubunchu/">"Ubunchu!"</a> by Hiroshi Seo is licensed under <a href="https://creativecommons.org/licenses/by-nc/3.0/">CC BY-NC 3.0</a>.</small></em>
</p>

## Installation

Download the latest release for your device from the [releases page](https://github.com/hanatsumi/rakuyomi/releases). Currently, only Kindle devices are supported (the Kindle build should work with Kobo devices, but it's untested).

Place the `rakuyomi.koplugin` folder inside KOReader's `plugins` folder.

## Configuration

In order to use the plugin, you'll need to add some sources. rakuyomi is compatible with [Aidoku](https://github.com/Aidoku/Aidoku) sources. The easiest way to obtain sources is to configure _source lists_ and download the sources through the app.

Create a `rakuyomi/settings.json` file inside KOReader's home directory, with the following contents:

```json
{
    "$schema": "https://github.com/hanatsumi/rakuyomi/releases/download/main/settings.schema.json",
    "source_lists": [
        "<source list URL>",
    ],
    "languages": ["en", "<your preferred language here>"]
}
```

After configuring a source list, sources can be installed through the plugin, by going to the main menu and going to "Manage sources".

## Usage

To open the library view, go to the "Search" menu and tap on the "Rakuyomi" entry. Tap the menu button on the top left to open the main menu.
