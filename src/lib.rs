mod tests {
    #[test]
    fn deps_installed() {
        assert!(installed());
    }
}

fn installed() -> bool {
    false
}
