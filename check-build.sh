set -e
set -x

echo "checks that this builds on std+no_std + that all tests run + that all features compile"
cargo build --all-targets

cargo test --all-targets

cargo fmt -- --check # (--check doesn't change the files)

cargo doc

cargo clippy --all-targets

# test 32-bit build (because AT-values are usize)
rustup target add i686-unknown-linux-musl
cargo build --target i686-unknown-linux-musl
cargo test --target i686-unknown-linux-musl

# test some random no_std target
rustup target add thumbv7em-none-eabihf
cargo build --target thumbv7em-none-eabihf
