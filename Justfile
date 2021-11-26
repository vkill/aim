default:
  @just --list

build:
    #!/bin/bash
    cargo build  --verbose --all || exit 1
    for d in $(find test -type d); do
        pushd $d > /dev/null
            [ -f Justfile ] && just build
        popd > /dev/null
    done

test: build
    #!/bin/bash
    cargo test  --verbose --all || exit 1

    for d in $(find test -type d); do
        pushd $d > /dev/null
            [ -f Justfile ] && just test
        popd > /dev/null
    done
