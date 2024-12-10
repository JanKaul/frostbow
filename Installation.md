# Installation

## MacOS

### Homebrew

```bash
brew tap jankaul/frostbow
brew install jankaul/frostbow/frostbow
```

### Download

X86:
```bash
wget -qO- https://github.com/jankaul/frostbow/releases/download/v0.1.0/frostbow-macOS-x86_64.tar.gz | tar xvz
```

Arm:
```bash
wget -qO- https://github.com/jankaul/frostbow/releases/download/v0.1.0/frostbow-macOS-aarch64.tar.gz | tar xvz
```

## Linux

```bash
wget -qO- https://github.com/jankaul/frostbow/releases/download/v0.1.0/frostbow-linux-x86_64.tar.gz | tar xvz
```

## Windows

[https://github.com/jankaul/frostbow/releases/download/v0.1.0/frostbow-windows-x86_64.zip](https://github.com/jankaul/frostbow/releases/download/v0.1.0/frostbow-windows-x86_64.zip)

## Cargo

Warning: Until the `object_store` crate version 0.11.2 is released, the crates.io version of frostbow cannot be used with the Filesystem catalog.

```bash
cargo install frostbow
```