# maker-util

A lightweight shell script that makes calling the Maker API endpoint more ergonomic.

![Shell](https://img.shields.io/badge/shell-bash-blue)

## Installation

Clone the repository and make the script executable:

```bash
git clone git@github.com:brianchli/maker.git
cd maker-util
chmod +x maker-util.sh
```

Move it to a directory in your `PATH` for system-wide access (optional):

```bash
cp maker-util.sh /usr/local/bin/maker-util
# or 
# ln -s $(pwd)/maker-util.sh /usr/local/bin/maker-util
```

**Prerequisites:** `curl` and `jq` and `gum` must be installed. On MacOS with
Homebrew:

```bash
brew install jq curl gum
```

## Usage

Call the script with the current available endpoints:

```bash
maker-util create
maker-util models
```
