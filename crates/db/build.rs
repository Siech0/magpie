//! Rebuilds when migrations change, since `sqlx::migrate!` cannot track them on stable.

fn main() {
    println!("cargo:rerun-if-changed=migrations");
}
