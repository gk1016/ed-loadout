fn main() {
    println!("{}", greeting());
}

fn greeting() -> &'static str {
    "ed-loadout: created by bts-repo-init"
}

#[cfg(test)]
mod tests {
    #[test]
    fn greeting_names_the_repo() {
        assert!(super::greeting().starts_with("ed-loadout"));
    }
}
