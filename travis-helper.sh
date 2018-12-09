#!/bin/bash

action="$1"

# Run unit and integration tests.
if [ "$action" = "test" ]; then
  if [ $DEFAULT_FEATURES = true ]; then
    if [ -z $FEATURES ]; then
      cargo test --verbose --no-default-features
    else
      cargo test --verbose --no-default-features --features="$FEATURES"
    fi
  else
    if [ -z $FEATURES ]; then
      cargo test --verbose
    else
      cargo test --verbose --features="$FEATURES"
    fi
  fi

# Check formatting.
elif [ "$action" = "fmt_check" ]; then
  if [ "$TRAVIS_RUST_VERSION" = "stable" && (-z $FEATURES) && $DEFAULT_FEATURES ]]; then
    rustup component add rustfmt &&
    cargo fmt --verbose -- --check
  fi

# Run Clippy.
elif [ "$action" = "clippy_check" ]; then
  if [ "$TRAVIS_RUST_VERSION" = "stable" ]; then
    rustup component add clippy &&
    cargo clippy --verbose
  fi

# Upload development documentation for the develop branch.
elif [ "$action" = "documentation" ]; then
  if [ "$TRAVIS_BRANCH" = "develop" ]
    cargo doc -v --document-private-items &&
    echo "<meta http-equiv=refresh content=0;url=os_balloon/index.html>" > target/doc/index.html
  fi

# Upload code coverage report for stable builds in Linux.
elif [ "$action" = "upload_code_coverage" ]; then
  if [[ "$TRAVIS_BUILD_STAGE_NAME" == "Test" &&
        "$TRAVIS_RUST_VERSION" == "stable" &&
        (-z $FEATURES) && $DEFAULT_FEATURES ]]; then
    wget https://github.com/SimonKagstrom/kcov/archive/master.tar.gz &&
    tar xzf master.tar.gz &&
    cd kcov-master &&
    mkdir build &&
    cd build &&
    cmake .. &&
    make &&
    sudo make install &&
    cd ../.. &&
    rm -rf kcov-master &&
    for file in target/debug/os_balloon-*[^\.d]; do
      mkdir -p "target/cov/$(basename $file)";
      kcov --exclude-pattern=/.cargo,/usr/lib --verify "target/cov/$(basename $file)" "$file";
    done &&
    for file in target/debug/launcher-*[^\.d]; do
      mkdir -p "target/cov/$(basename $file)";
      kcov --exclude-pattern=/.cargo,/usr/lib --verify "target/cov/$(basename $file)" "$file";
    done &&
    bash <(curl -s https://codecov.io/bash) &&
    echo "Uploaded code coverage"
  fi

fi
exit $?