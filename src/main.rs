use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::collections::HashMap;
use std::time::Instant;

// Represent a die with its maximum value and current value
#[derive(Debug, Clone)]
struct Die {
    max_value: u8,
    current_value: u8,
}

impl Die {
    fn new(max_value: u8) -> Self {
        Die {
            max_value,
            current_value: 0,
        }
    }

    fn roll(&mut self, rng: &mut impl Rng) {
        // Use the new method name from rand 0.9.1
        self.current_value = rng.random_range(1..=self.max_value);
    }

    fn points(&self) -> u8 {
        self.max_value - self.current_value
    }
}

// Game state
struct Game {
    dice: Vec<Die>,
}

impl Game {
    fn new() -> Self {
        let mut dice = Vec::new();

        // Add the 12 six-sided dice
        for _ in 0..12 {
            dice.push(Die::new(6));
        }

        // Add the special dice
        dice.push(Die::new(8));
        dice.push(Die::new(9));
        dice.push(Die::new(12));

        Game { dice }
    }

    fn roll_all(&mut self, rng: &mut impl Rng) {
        for die in &mut self.dice {
            die.roll(rng);
        }
    }

    fn remove_dice(&mut self, indices: &[usize]) -> u8 {
        // Sort indices in descending order to avoid shifting problems
        let mut sorted_indices = indices.to_vec();
        sorted_indices.sort_unstable_by(|a, b| b.cmp(a));

        let mut points = 0;
        for &index in &sorted_indices {
            if index < self.dice.len() {
                points += self.dice[index].points();
                self.dice.remove(index);
            }
        }
        points
    }

    fn is_over(&self) -> bool {
        self.dice.is_empty()
    }
}

// Define strategy as a type alias for a function that selects dice to remove
type Strategy = fn(&[Die]) -> Vec<usize>;

// Strategy functions - each returns indices of dice to remove

// Find all dice with zero points (maximum value rolled)
fn find_zero_point_dice(dice: &[Die]) -> Vec<usize> {
    dice.iter()
        .enumerate()
        .filter(|(_, die)| die.points() == 0)
        .map(|(i, _)| i)
        .collect()
}

// Find the single die with minimum points
fn find_min_points_die(dice: &[Die]) -> usize {
    let mut min_points = u8::MAX;
    let mut min_index = 0;

    for (i, die) in dice.iter().enumerate() {
        let points = die.points();
        if points < min_points {
            min_points = points;
            min_index = i;
        }
    }

    min_index
}

// Find all dice with the same value as the die at the given index
fn find_dice_with_same_value(dice: &[Die], index: usize) -> Vec<usize> {
    if index >= dice.len() {
        return vec![];
    }

    let target_value = dice[index].current_value;

    dice.iter()
        .enumerate()
        .filter(|(_, die)| die.current_value == target_value)
        .map(|(i, _)| i)
        .collect()
}

// Combining these into complete strategies

// Strategy 1: Remove the die with minimum points
fn min_points_strategy(dice: &[Die]) -> Vec<usize> {
    vec![find_min_points_die(dice)]
}

// Strategy 2: Remove all dice with zero points, or the min points die if none
fn zero_points_strategy(dice: &[Die]) -> Vec<usize> {
    let zero_indices = find_zero_point_dice(dice);
    if !zero_indices.is_empty() {
        zero_indices
    } else {
        vec![find_min_points_die(dice)]
    }
}

// Strategy 3: Remove all dice with the same value as the min points die
fn same_value_strategy(dice: &[Die]) -> Vec<usize> {
    let min_index = find_min_points_die(dice);
    find_dice_with_same_value(dice, min_index)
}

// Strategy 4: Prioritize removing high-sided dice when they have low points
fn prioritize_max_value_strategy(dice: &[Die]) -> Vec<usize> {
    // First check for zero point dice
    let zero_indices = find_zero_point_dice(dice);
    if !zero_indices.is_empty() {
        return zero_indices;
    }

    // Find the die with best score (higher max_value and lower points)
    let mut best_index = 0;
    let mut best_score = f32::MIN;

    for (i, die) in dice.iter().enumerate() {
        // Score function: higher is better - prioritize high max_value and low points
        let score = die.max_value as f32 - 2.0 * die.points() as f32;
        if score > best_score {
            best_score = score;
            best_index = i;
        }
    }

    vec![best_index]
}

// Strategy 5: Minimize expected future regret
fn minimize_regret_strategy(dice: &[Die]) -> Vec<usize> {
    // First check for zero point dice
    let zero_indices = find_zero_point_dice(dice);
    if !zero_indices.is_empty() {
        return zero_indices;
    }

    // Calculate expected future regret for each die
    let mut min_regret = f32::MAX;
    let mut min_index = 0;

    for (i, die) in dice.iter().enumerate() {
        // Expected value from removing this die
        let removal_points = die.points() as f32;

        // Expected value from average future rolls of this die
        // The average roll on an n-sided die is (n+1)/2
        let expected_future_value = (die.max_value as f32 + 1.0) / 2.0;
        let expected_future_points = die.max_value as f32 - expected_future_value;

        // Regret = current points - expected future points
        let regret = removal_points - expected_future_points;

        if regret < min_regret {
            min_regret = regret;
            min_index = i;
        }
    }

    vec![min_index]
}

// Strategy 6: Hybrid - combine min points and prioritize max value
fn hybrid_strategy(dice: &[Die]) -> Vec<usize> {
    // First check for zero point dice
    let zero_indices = find_zero_point_dice(dice);
    if !zero_indices.is_empty() {
        return zero_indices;
    }

    // For each die, calculate a weighted score based on points and max value
    let mut best_index = 0;
    let mut best_score = f32::MAX; // Lower is better here

    for (i, die) in dice.iter().enumerate() {
        // Weight low points more heavily for low-sided dice
        // and give more importance to the die's relative value (how close to max)
        let relative_value = die.current_value as f32 / die.max_value as f32;
        let score = die.points() as f32 * (1.0 + (1.0 - relative_value));

        if score < best_score {
            best_score = score;
            best_index = i;
        }
    }

    vec![best_index]
}

fn simulate_game(strategy: Strategy, seed: u64) -> u8 {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut game = Game::new();
    let mut total_points = 0;

    while !game.is_over() {
        game.roll_all(&mut rng);
        let indices = strategy(&game.dice);
        total_points += game.remove_dice(&indices);
    }

    total_points
}

fn run_simulations(strategy: Strategy, num_simulations: usize) -> (f64, u8, u8) {
    let mut total_points = 0;
    let mut min_points = u8::MAX;
    let mut max_points = 0;

    for i in 0..num_simulations {
        let points = simulate_game(strategy, i as u64);
        total_points += points as u64;
        min_points = min_points.min(points);
        max_points = max_points.max(points);
    }

    let avg_points = total_points as f64 / num_simulations as f64;
    (avg_points, min_points, max_points)
}

fn main() {
    let num_simulations = 100000;

    // Define the strategies with their names
    let strategies: Vec<(String, Strategy)> = vec![
        ("Min Points".to_string(), min_points_strategy),
        ("Zero Points".to_string(), zero_points_strategy),
        ("Same Value".to_string(), same_value_strategy),
        (
            "Prioritize Max Value".to_string(),
            prioritize_max_value_strategy,
        ),
        ("Minimize Regret".to_string(), minimize_regret_strategy),
        ("Hybrid".to_string(), hybrid_strategy),
    ];

    println!("Simulating {} games for each strategy...", num_simulations);

    // Type alias for the result type to reduce complexity
    type SimulationResult = (f64, u8, u8, std::time::Duration);

    let mut results = HashMap::new();

    for (name, strategy) in strategies {
        let start = Instant::now();
        let (avg_points, min_points, max_points) = run_simulations(strategy, num_simulations);
        let duration = start.elapsed();

        results.insert(name, (avg_points, min_points, max_points, duration));
    }

    // Print results in a nicely formatted table
    println!(
        "\n{:<25} {:<15} {:<10} {:<10} {:<10}",
        "Strategy", "Avg Points", "Min", "Max", "Time"
    );
    println!("{:-<72}", "");

    // Sort and display results
    let mut sorted_results: Vec<(&String, &SimulationResult)> = results.iter().collect();
    sorted_results.sort_by(|a, b| a.1.0.partial_cmp(&b.1.0).unwrap());

    for (name, (avg, min, max, duration)) in sorted_results {
        println!(
            "{:<25} {:<15.2} {:<10} {:<10} {:.2?}",
            name, avg, min, max, duration
        );
    }
}
