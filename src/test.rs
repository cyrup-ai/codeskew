use std::collections::HashMap;

/// A simple example function that demonstrates
/// multiple Rust features in a compact space.
/// This will render better in the circular format.
fn main() {
    // Create a vector of integers
    let numbers = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    
    // Use iterators and closures
    let sum: i32 = numbers.iter().sum();
    let squares: Vec<i32> = numbers.iter()
        .map(|&x| x * x)
        .collect();
    
    // Print the results
    println!("Sum of numbers: {}", sum);
    println!("Squares: {:?}", squares);
    
    // Create a HashMap with some data
    let mut map = HashMap::new();
    map.insert("one", 1);
    map.insert("two", 2);
    map.insert("three", 3);
    
    // Iterate over the map
    for (key, value) in &map {
        println!("{}: {}", key, value);
    }
    
    // Pattern matching example
    for num in numbers {
        match num {
            1 => println!("Found one!"),
            n if n % 2 == 0 => println!("{} is even", n),
            n => println!("{} is odd", n),
        }
    }
}
