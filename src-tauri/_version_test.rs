use semver::Version;
fn main() {
    let pairs = [
        ("2.0.1-Beta.2", "2.0.1-beta2"),
        ("2.0.1-Beta.2", "2.0.1-Beta.2"),
        ("2.0.1", "2.0.1-beta2"),
        ("2.0.1", "2.0.1"),
    ];
    for (a,b) in pairs {
        let va = Version::parse(a).unwrap_or_else(|e| panic!("{}: {}", a, e));
        let vb = Version::parse(b).unwrap_or_else(|e| panic!("{}: {}", b, e));
        println!("{a} vs {b} => eq={}, gt={}", va == vb, vb > va);
    }
}
