#!/bin/bash
set -e

if [ -n "$1" ] && [ "$1" -eq 1 ]; then
	rustup override set 1.70.0-x86_64-unknown-linux-gnu
	cargo build --target mipsel-unknown-linux-gnu --release
	cp target/mipsel-unknown-linux-gnu/release/sample_mips /var/nfs/public/ -v

else
	#rustup override set nightly
	#rustup override set nightly-2021-11-22-x86_64-unknown-linux-gnu
	cargo build -Zbuild-std --target mipsel-unknown-linux-uclibc --release
	cp target/mipsel-unknown-linux-uclibc/release/sample_mips /var/nfs/public/ -v
fi

set +e
