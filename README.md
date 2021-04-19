# cbzlint

[![License](https://img.shields.io/badge/License-BSD%203--Clause-blue.svg)](https://opensource.org/licenses/BSD-3-Clause)

cbzlint is a command-line tool that allows you to check the correctness of CBZ
file names.

## Performed checks

- Check image resolution
- Check publication year
- Check authors list

## How to install

You can download a pre-compiled executable for Linux, MacOS and Windows
operating systems
[on the release page](https://github.com/TehUncleDolan/cbzlint/releases/latest),
then you should copy that executable to a location from your `$PATH` env.

You might need to run `chmod +x cbzlint_amd64` or `chmod +x cbzlint_darwin`.

## Usage

The simplest invocation only requires you to specify the files you want to
check.

```bash
cbzlint my-book.cbz another.book.cbz
```

You can also specify a directory.

```bash
cbzlint my-series/
```

Or mix both:

```bash
cbzlint my-series/ my-oneshot.cbz another-series/
```
