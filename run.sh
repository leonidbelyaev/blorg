export ROCKET_SECRET_KEY=$(openssl rand -base64 32)
cargo run --release
