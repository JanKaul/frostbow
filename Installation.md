# Installation

## MacOS

#### Homebrew

```bash
brew tap jankaul/frostbow
brew install jankaul/frostbow/frostbow
```

#### Download

X86:
```bash
wget -qO- https://github.com/JanKaul/frostbow/releases/download/v0.2.0/frostbow-macOS-x86_64.tar.gz | tar xvz
```

Arm:
```bash
wget -qO- https://github.com/JanKaul/frostbow/releases/download/v0.2.0/frostbow-macOS-arm64.tar.gz | tar xvz
```

## Linux

```bash
wget -qO- https://github.com/JanKaul/frostbow/releases/download/v0.2.0/frostbow-Linux-gnu-x86_64.tar.gz | tar xvz
```

## Windows

[https://github.com/JanKaul/frostbow/releases/download/v0.2.0/frostbow-Windows-msvc-x86_64.zip](https://github.com/JanKaul/frostbow/releases/download/v0.2.0/frostbow-Windows-msvc-x86_64.zip)

## Cargo

Warning: Until the `object_store` crate version 0.11.2 is released, the crates.io version of frostbow cannot be used with the Filesystem catalog.

```bash
cargo install frostbow
```