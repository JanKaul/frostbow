#!/bin/bash

VER=v0.2.0

OS=$(uname -s)
ARCH=$(uname -m)

command -v wget>/dev/null 2>&1 || { echo >&2 "Required tool wget could not be found. Aborting."; exit 1; }
command -v tar>/dev/null 2>&1 || { echo >&2 "Required tool tar could not be found. Aborting."; exit 1; }

BIN="~/.local/bin"

if [ "${OS}" = "Linux" ]
then
    if [ "${ARCH}" = "x86_64" ] || [ "${ARCH}" = "amd64" ]
    then
        DIST=Linux-gnu-x86_64
    fi
elif [ "${OS}" = "Darwin" ]
then
    if [ "${ARCH}" = "x86_64" ] || [ "${ARCH}" = "amd64" ]
    then
        DIST=macOS-x86_64
    elif [ "${ARCH}" = "aarch64" ] || [ "${ARCH}" = "arm64" ]
    then
        DIST=macOS-arm64
    fi
fi

if [ -z "${DIST}" ]
then
    echo "Operating system '${OS}' / architecture '${ARCH}' is unsupported." 1>&2
    exit 1
fi

URL="https://github.com/JanKaul/frostbow/releases/download/$VER/frostbow-$DIST.tar.gz"

wget -qO- $URL | tar xvz || fail "download failed"

# install frostbow $BIN
