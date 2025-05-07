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
        // Use the updated method name to avoid deprecation warning
        self.current_value = rng.gen_range(1..=self.max_value);
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

// Strategy trait for different dice removal strategies
trait RemovalStrategy {
    fn name(&self) -> &str;
    fn select_dice(&self, dice: &[Die]) -> Vec<usize>;
}

// Strategy: Remove the die with lowest points (greedy approach)
struct MinPointsStrategy;

impl RemovalStrategy for MinPointsStrategy {
    fn name(&self) -> &str {
        "Min Points"
    }

    fn select_dice(&self, dice: &[Die]) -> Vec<usize> {
        let mut min_points = u8::MAX;
        let mut min_index = 0;

        for (i, die) in dice.iter().enumerate() {
            let points = die.points();
            if points < min_points {
                min_points = points;
                min_index = i;
            }
        }

        vec![min_index]
    }
}

// Strategy: Remove all dice with zero points (optimal when available)
struct ZeroPointsStrategy;

impl RemovalStrategy for ZeroPointsStrategy {
    fn name(&self) -> &str {
        "Zero Points"
    }

    fn select_dice(&self, dice: &[Die]) -> Vec<usize> {
        let zero_indices: Vec<usize> = dice
            .iter()
            .enumerate()
            .filter(|(_, die)| die.points() == 0)
            .map(|(i, _)| i)
            .collect();

        if !zero_indices.is_empty() {
            zero_indices
        } else {
            // Fallback to min points if no zeros
            let mut min_points = u8::MAX;
            let mut min_index = 0;

            for (i, die) in dice.iter().enumerate() {
                let points = die.points();
                if points < min_points {
                    min_points = points;
                    min_index = i;
                }
            }

            vec![min_index]
        }
    }
}

// Strategy: Prioritize removing max value dice (focusing on high-sided dice first)
struct PrioritizeMaxValueStrategy;

impl RemovalStrategy for PrioritizeMaxValueStrategy {
    fn name(&self) -> &str {
        "Prioritize Max Value"
    }

    fn select_dice(&self, dice: &[Die]) -> Vec<usize> {
        // First find any zero point dice (always optimal to take)
        let zero_indices: Vec<usize> = dice
            .iter()
            .enumerate()
            .filter(|(_, die)| die.points() == 0)
            .map(|(i, _)| i)
            .collect();

        if !zero_indices.is_empty() {
            return zero_indices;
        }

        // Otherwise, find the die with the highest max_value and lowest points
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
}

// Strategy: Minimize expected future regret
struct MinimizeRegretStrategy;

impl RemovalStrategy for MinimizeRegretStrategy {
    fn name(&self) -> &str {
        "Minimize Regret"
    }

    fn select_dice(&self, dice: &[Die]) -> Vec<usize> {
        // First check for zero point dice
        let zero_indices: Vec<usize> = dice
            .iter()
            .enumerate()
            .filter(|(_, die)| die.points() == 0)
            .map(|(i, _)| i)
            .collect();

        if !zero_indices.is_empty() {
            return zero_indices;
        }

        // Calculate expected future regret for each die
        // Lower is better - we'll pick the die with minimum expected future regret
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
}

// Strategy: Remove all dice with the same current value as min points die
struct RemoveSameValueStrategy;

impl RemovalStrategy for RemoveSameValueStrategy {
    fn name(&self) -> &str {
        "Remove Same Value"
    }

    fn select_dice(&self, dice: &[Die]) -> Vec<usize> {
        // Find die with minimum points
        let mut min_points = u8::MAX;
        let mut min_index = 0;

        for (i, die) in dice.iter().enumerate() {
            let points = die.points();
            if points < min_points {
                min_points = points;
                min_index = i;
            }
        }

        // Get the value we want to match
        let target_value = dice[min_index].current_value;

        // Collect all dice with this value
        let indices: Vec<usize> = dice
            .iter()
            .enumerate()
            .filter(|(_, die)| die.current_value == target_value)
            .map(|(i, _)| i)
            .collect();

        indices
    }
}

fn simulate_game(strategy: &(impl RemovalStrategy + ?Sized), seed: u64) -> u8 {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut game = Game::new();
    let mut total_points = 0;

    while !game.is_over() {
        game.roll_all(&mut rng);
        let indices = strategy.select_dice(&game.dice);
        total_points += game.remove_dice(&indices);
    }

    total_points
}

fn run_simulations(
    strategy: &(impl RemovalStrategy + ?Sized),
    num_simulations: usize,
) -> (f64, u8, u8) {
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
    let strategies: Vec<Box<dyn RemovalStrategy>> = vec![
        Box::new(MinPointsStrategy),
        Box::new(ZeroPointsStrategy),
        Box::new(PrioritizeMaxValueStrategy),
        Box::new(MinimizeRegretStrategy),
        Box::new(RemoveSameValueStrategy),
    ];

    println!("Simulating {} games for each strategy...", num_simulations);

    let mut results = HashMap::new();

    for strategy in strategies {
        let start = Instant::now();
        let (avg_points, min_points, max_points) = run_simulations(&*strategy, num_simulations);
        let duration = start.elapsed();

        results.insert(
            strategy.name().to_string(),
            (avg_points, min_points, max_points, duration),
        );
    }

    // Print results in a nicely formatted table
    println!(
        "\n{:<25} {:<15} {:<10} {:<10} {:<10}",
        "Strategy", "Avg Points", "Min", "Max", "Time"
    );
    println!("{:-<72}", "");

    let mut sorted_results: Vec<(&String, &(f64, u8, u8, std::time::Duration))> =
        results.iter().collect();
    sorted_results.sort_by(|a, b| a.1.0.partial_cmp(&b.1.0).unwrap());

    for (name, (avg, min, max, duration)) in sorted_results {
        println!(
            "{:<25} {:<15.2} {:<10} {:<10} {:.2?}",
            name, avg, min, max, duration
        );
    }
}
