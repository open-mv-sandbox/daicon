#!/bin/bash
set -e

cargo build

export CMD="./target/debug/daicon-tools"
export TARGET="--target ./package.example-daicon"

echo -e "\n# Creating Package"

$CMD create $TARGET
$CMD set $TARGET --id bacc2ba1-8dc7-4d54-a7a4-cdad4d893a1b --input ./README.md
$CMD set $TARGET --id 46570f62-7f7f-451d-b7d3-01add821ab9c --input ./LICENSE-APACHE
$CMD set $TARGET --id 1f063ad4-5a91-47fe-b95c-668fc41a719d --input ./LICENSE-MIT

echo -e "\n# Getting Example from Package"

$CMD get $TARGET --id bacc2ba1-8dc7-4d54-a7a4-cdad4d893a1b --output /dev/stdout
$CMD get $TARGET --id 1f063ad4-5a91-47fe-b95c-668fc41a719d --output /dev/stdout
