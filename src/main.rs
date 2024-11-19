fn main() {
    println!("Hello, world!")
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }

    #[test]
    #[should_panic]
    fn it_fails() {
        assert_eq!(2 + 2, 5);
    }
}
