# Rustify

Music player complete with Discord RPC and Last.fm scrobbling!
<img width="1276" height="795" alt="image" src="https://github.com/user-attachments/assets/2fb96eee-2fee-467a-8646-ae4f8d07366b" />
<img width="1278" height="796" alt="image" src="https://github.com/user-attachments/assets/cd0c18d3-b5dc-4dd5-90d7-8e17030f48e9" />
<img width="512" height="207" alt="image" src="https://github.com/user-attachments/assets/492dd326-2c43-4d73-b215-7c0f154e3bcb" />
<img width="891" height="103" alt="image" src="https://github.com/user-attachments/assets/9b1e7794-f348-4f9f-b5ab-96ec590fe7bf" />

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

## Next Steps

Once downloaded, you can open a folder and start playing music! You can also link your last.fm account through the settings page and setting your username!
