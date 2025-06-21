#!/bin/bash

set -e

cat << EOF
Package: mcping
Version: $1
Maintainer: valkyrie_pilot <valk@randomairborne.dev>
Depends: libc6
Architecture: $2
Homepage: https://github.com/randomairborne/mcping
Description: Minecraft ping HTTP api
EOF
