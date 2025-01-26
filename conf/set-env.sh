echo "project dir is ${PROJECT_DIR}"

rustupHomeDir="${PROJECT_DIR}/.rustup"
mkdir -p "${rustupHomeDir}"
export RUSTUP_HOME="${rustupHomeDir}"

cargoHomeDir="${PROJECT_DIR}/.cargo"
clangBinary=$(which clang)
moldBinary=$(which mold)
mkdir -p "${cargoHomeDir}"
cat > "${PROJECT_DIR}/.cargo/config.toml" <<EOF
[target.x86_64-unknown-linux-gnu]
linker = "$clangBinary"
rustflags = ["-C", "link-arg=--ld-path=$moldBinary"]

[target.aarch64-apple-darwin]
linker = "$clangBinary"
EOF