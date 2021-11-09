cargo build --release && \
cargo build --release --target x86_64-pc-windows-gnu && \
cp target/x86_64-pc-windows-gnu/release/vrc_gatekeeper.exe /mnt/d/dev_tmp/rust/ && \
echo done.

