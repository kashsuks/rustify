# Rustify

## Running Release Builds

Release binaries are distributed as compressed archives.

### macOS

Download `rustify-macos-universal.zip`, then run:

```sh
unzip rustify-macos-universal.zip
chmod +x rustify
./rustify
```

The macOS binary is universal and supports both Apple Silicon and Intel Macs.

Because the binary is not code-signed, macOS may block it the first time you
open it. If that happens, remove the quarantine flag and run it again:

```sh
xattr -dr com.apple.quarantine rustify
./rustify
```

### Windows

Download `rustify-windows-x86_64.zip`, extract it, then run `rustify.exe`.

Because the executable is not code-signed, Windows SmartScreen may show a
warning the first time it runs. Choose **More info**, then **Run anyway** if you
trust the build.

### Arch Linux

Download `rustify-arch-linux-x86_64.tar.gz`, then run:

```sh
tar -xzf rustify-arch-linux-x86_64.tar.gz
chmod +x rustify
./rustify
```

The Arch Linux build expects ALSA to be available:

```sh
sudo pacman -S --needed alsa-lib
```
