#!/bin/bash

cargo --frozen metadata --no-deps --format-version 1 | jq -r '.packages.[] | select(.name=="mcping").version'
