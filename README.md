# ESP Tools

Reads the statics from a ESP file and packages the ESP, meshes, and textures into a single zip. I use this for submitting files to the Tamriel Rebuilt website.

![demonstration](assets/demonstration.gif)

# Installation

On Windows, download and install the msi file from the [releases](https://github.com/nicholas477/esp-tools/releases) page.

# How to use

On Windows, right click your ESP file and select "Run esp-tools". A zip file will be produced with the same name as the esp file, in the same directory.

There's also a command line interface:

# Command line options

<!-- BEGIN GENERATED HELP -->
```text
Grabs the statics from an ESP file, then packages the meshes, textures, and the ESP file into a single zip.

Usage: esp-tools.exe [OPTIONS] [FILE]

Arguments:
  [FILE]  ESP file to isolate meshes and textures from

Options:
  -o, --output <OUTPUT>       (Optional) Output file path. If not specified, the zip file will be created in the same directory as the input ESP file
  -p, --pause                 Pause before exiting. If specified, the program will wait for user input before exiting
  -u, --update                Update the program to the latest version.
                              If specified, all other arguments except --pause will be ignored, and the program will check for updates and apply them if available.
  -i, --include-master-files  Include assets that are also referenced by master plugins.
                              If specified, the program will include assets that are referenced by master files into the zip file.
                              By default, assets that are referenced by master files will be excluded from the zip file.
  -h, --help                  Print help
  -V, --version               Print version
```
<!-- END GENERATED HELP -->
