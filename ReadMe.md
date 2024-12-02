# Some Command line tools for Android development

## ApkInfo: A tool to extract information from APK files

```
Usage: apkinfo --apk-path <APK_PATH>

Options:
  -a, --apk-path <APK_PATH>  Apk File Path
  -h, --help                 Print help
  -V, --version              Print version
```

## Apkex: Apk tool command line tool

```
Usage: apkex [OPTIONS] --cmd <CMD> --file-or-folder-path <FILE_OR_FOLDER_PATH>

Options:
  -c, --cmd <CMD>                                  Command [u: unpack; p: pack;]
  -f, --file-or-folder-path <FILE_OR_FOLDER_PATH>  File Or Folder Path
  -k, --key-config <KEY_CONFIG>                    Keystore config file path [default: ]
  -h, --help                                       Print help
  -V, --version                                    Print version
```

## AdbLog: start apk and logcat

Usage: adblog [OPTIONS] --file-path <FILE_PATH>

Options:
  -c, --cmd <CMD>              Command [i: install apk; r: reinstall apk; mem: watch memory; cpu: watch cpu;] [default: i]
  -f, --file-path <FILE_PATH>  Apk or Apks or Aab File Path
  -h, --help                   Print help
  -V, --version                Print version




