#!/bin/bash
set -e

cargo build

export CMD="./target/debug/daicon-tools"
export TARGET="--target ./package.example"

echo -e "\n# Creating Package"

$CMD create $TARGET
$CMD set $TARGET --id 0xbacc2ba1 --input ./README.md
$CMD set $TARGET --id 0x46570f62 --input ./LICENSE-APACHE
$CMD set $TARGET --id 0x1f063ad4 --input ./LICENSE-MIT

echo -e "\n# Getting Example from Package"

$CMD get $TARGET --id 0xbacc2ba1 --output /dev/stdout
$CMD get $TARGET --id 0x1f063ad4 --output /dev/stdout
