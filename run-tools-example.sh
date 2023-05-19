#!/bin/bash
set -e

cargo build

export CMD="./target/debug/daicon-tools"
export TARGET="--target ./package.example"

echo -e "\n# Creating Package"

$CMD create $TARGET
$CMD set $TARGET --id 0xbacc2ba18dc74d54 --input ./README.md
$CMD set $TARGET --id 0x46570f627f7f451d --input ./LICENSE-APACHE
$CMD set $TARGET --id 0x1f063ad45a9147fe --input ./LICENSE-MIT

echo -e "\n# Getting Example from Package"

$CMD get $TARGET --id 0xbacc2ba18dc74d54 --output /dev/stdout
$CMD get $TARGET --id 0x1f063ad45a9147fe --output /dev/stdout
