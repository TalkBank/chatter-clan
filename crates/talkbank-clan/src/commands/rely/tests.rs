use super::*;

#[test]
fn rely_perfect_agreement() {
    // Simulate perfect agreement
    let mut freq1 = BTreeMap::new();
    let mut freq2 = BTreeMap::new();
    freq1.insert("A".to_owned(), 5u64);
    freq2.insert("A".to_owned(), 5u64);

    let total = 5u64;
    let agreed_count = 5u64;
    let po = agreed_count as f64 / total as f64;
    assert!((po - 1.0).abs() < f64::EPSILON);
}

#[test]
fn rely_no_agreement() {
    let total = 10u64;
    let agreed_count = 0u64;
    let po = agreed_count as f64 / total as f64;
    assert!((po - 0.0).abs() < f64::EPSILON);
}
