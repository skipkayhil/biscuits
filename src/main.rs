use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, Clone)]
struct Die {
    max_value: u8,
    points: u8,
}

impl Die {
    fn six() -> Self {
        Die {
            max_value: 6,
            points: 6,
        }
    }

    fn eight() -> Self {
        Die {
            max_value: 8,
            points: 8,
        }
    }

    fn nine() -> Self {
        Die {
            max_value: 9,
            points: 9,
        }
    }

    fn twelve() -> Self {
        Die {
            max_value: 12,
            points: 12,
        }
    }

    #[cfg(test)]
    fn with_points(mut self, points: u8) -> Self {
        self.points = points;
        self
    }

    fn roll(&mut self, rng: &mut impl Rng) {
        self.points = rng.random_range(0..self.max_value);
    }

    fn points(&self) -> u8 {
        self.points
    }
}

#[cfg(test)]
mod die_tests {
    use super::*;

    #[test]
    fn test_points() {
        let mut die = Die::six();
        die.points = 2;
        assert_eq!(die.points(), 2); // 6 - 4 = 2 points

        die.points = 0;
        assert_eq!(die.points(), 0); // 6 - 6 = 0 points
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
            dice.push(Die::six());
        }

        // Add the special dice
        dice.push(Die::eight());
        dice.push(Die::nine());
        dice.push(Die::twelve());

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

impl std::fmt::Display for Game {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        for die in self.dice.iter() {
            write!(f, "{} ", die.points)?;
        }
        writeln!(f)?;
        for die in self.dice.iter() {
            write!(f, "{} ", die.max_value)?;
        }

        Ok(())
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

fn find_big_zero_dice(dice: &[Die]) -> Vec<usize> {
    dice.iter()
        .enumerate()
        .filter(|(_, die)| die.points() == 0 && die.max_value != 6)
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

fn find_big_min_die(dice: &[Die]) -> usize {
    dice.iter()
        .enumerate()
        .min_by_key(|(_, d)| (d.points(), u8::MAX - d.max_value))
        .unwrap()
        .0
}

// Remove the die with minimum points
fn one_min_strategy(dice: &[Die]) -> Vec<usize> {
    vec![find_min_points_die(dice)]
}

// Remove all dice with zero points, or the min points die if none
fn all_zero_or_one_min_strategy(dice: &[Die]) -> Vec<usize> {
    let zero_indices = find_zero_point_dice(dice);
    if !zero_indices.is_empty() {
        zero_indices
    } else {
        vec![find_min_points_die(dice)]
    }
}

// Prioritize removing high-sided dice when they have low points
fn all_zero_or_prio_min_strategy(dice: &[Die]) -> Vec<usize> {
    // First check for zero point dice
    let zero_indices = find_zero_point_dice(dice);
    if !zero_indices.is_empty() {
        return zero_indices;
    }

    // Find the die with best score (higher max_value and lower points)
    let mut best_index = 0;
    let mut best_score = i8::MIN;

    for (i, die) in dice.iter().enumerate() {
        // Score function: higher is better - prioritize high max_value and low points
        let score = die.max_value as i8 - 2 * die.points() as i8;
        if score > best_score {
            best_score = score;
            best_index = i;
        }
    }

    vec![best_index]
}

fn all_zero_or_big_min_strategy(dice: &[Die]) -> Vec<usize> {
    // First check for zero point dice
    let zero_indices = find_zero_point_dice(dice);
    if !zero_indices.is_empty() {
        return zero_indices;
    }

    vec![find_big_min_die(dice)]
}

fn all_big_zero_or_one_zero_or_big_min_strategy(dice: &[Die]) -> Vec<usize> {
    let big_zeros = find_big_zero_dice(dice);
    if !big_zeros.is_empty() {
        let big_dice_count = dice.iter().filter(|die| die.max_value != 6).count();
        let big_zero_count = big_zeros.len();

        if big_dice_count == big_zero_count {
            return find_zero_point_dice(dice);
        }
        return big_zeros;
    }

    let all_zeros = find_zero_point_dice(dice);
    if !all_zeros.is_empty() {
        if dice.iter().filter(|die| die.max_value != 6).count() == 0 {
            return all_zeros;
        } else {
            return vec![all_zeros[0]];
        }
    }

    vec![find_big_min_die(dice)]
}

#[cfg(test)]
mod func_tests {
    use super::*;

    #[test]
    fn test_find_zero_point_dice() {
        let dice = vec![
            Die::six().with_points(0),
            Die::six().with_points(3),
            Die::eight().with_points(0),
            Die::nine().with_points(1),
        ];

        let zero_indices = find_zero_point_dice(&dice);
        assert_eq!(zero_indices, vec![0, 2]); // Only indices 0 and 2 have zero points
    }

    #[test]
    fn test_find_min_points_die() {
        let dice = vec![
            Die::six().with_points(3),
            Die::eight().with_points(1),
            Die::nine().with_points(4),
            Die::twelve().with_points(2),
        ];

        let min_index = find_min_points_die(&dice);
        assert_eq!(min_index, 1); // Index 1 has only 1 point
    }

    #[test]
    fn test_all_zero_or_prio_min_strategy() {
        let mut dice = vec![
            Die::six().with_points(3),
            Die::eight().with_points(1),
            Die::nine().with_points(4),
            Die::twelve().with_points(2),
        ];

        let prio_min = all_zero_or_prio_min_strategy(&dice);
        assert_eq!(vec![3], prio_min); // 12 - 2 * 2 = 8

        dice.remove(prio_min[0]);
        let prio_min = all_zero_or_prio_min_strategy(&dice);
        assert_eq!(vec![1], prio_min); // 8 - 2 * 1 = 6

        dice.remove(prio_min[0]);
        let prio_min = all_zero_or_prio_min_strategy(&dice);
        assert_eq!(vec![1], prio_min); // 9 - 2 * 4 = 1

        dice.remove(prio_min[0]);
        let prio_min = all_zero_or_prio_min_strategy(&dice);
        assert_eq!(vec![0], prio_min); // 6 - 2 * 3 = 0
    }

    #[test]
    fn test_all_zero_or_prio_min_strategy_hmm() {
        let mut dice = vec![Die::six().with_points(1), Die::twelve().with_points(3)];

        let prio_min = all_zero_or_prio_min_strategy(&dice);
        assert_eq!(vec![1], prio_min); // 12 - 2 * 3 = 6

        dice.remove(prio_min[0]);
        let prio_min = all_zero_or_prio_min_strategy(&dice);
        assert_eq!(vec![0], prio_min); // 6 - 2 * 1 = 4
    }

    #[test]
    fn test_game_remove_dice() {
        let mut game = Game::new();
        game.dice = vec![
            Die::six().with_points(3),
            Die::eight().with_points(1),
            Die::nine().with_points(5),
            Die::twelve().with_points(2),
        ];

        let points = game.remove_dice(&[1, 3]);
        assert_eq!(points, 3); // 1 + 2 = 3 points
        assert_eq!(game.dice.len(), 2); // Should have 2 dice left
        assert_eq!(game.dice[0].points, 3); // First die should remain
        assert_eq!(game.dice[1].points, 5); // Third die should remain
    }

    #[test]
    fn test_full_game_simulation() {
        let points = simulate_game(one_min_strategy, 6454);
        assert_eq!(0, points);
    }
}

fn simulate_game(strategy: Strategy, seed: u64) -> u8 {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut game = Game::new();
    let mut total_points = 0;

    while !game.is_over() {
        game.roll_all(&mut rng);
        // if seed == 6454 {
        //     println!("{}\n", game);
        // }
        let indices = strategy(&game.dice);
        total_points += game.remove_dice(&indices);
    }

    total_points
}

fn run_simulations(strategy: Strategy, num_simulations: u64) -> (f64, u8, u64, u8) {
    let mut total_points = 0;
    let mut gravies = 0;
    let mut min_points = u8::MAX;
    let mut max_points = 0;

    for i in 0..num_simulations {
        let points = simulate_game(strategy, i);
        total_points += points as u64;
        if points == 0 {
            gravies += 1;
        }
        min_points = min_points.min(points);
        max_points = max_points.max(points);
    }

    let avg_points = total_points as f64 / num_simulations as f64;
    (avg_points, min_points, gravies, max_points)
}

fn main() {
    let num_simulations: u64 = 100000;

    let strategies: Vec<(String, Strategy)> = vec![
        ("One Min".to_string(), one_min_strategy),
        ("All Zero/One Min".to_string(), all_zero_or_one_min_strategy),
        (
            "All Zero/Prio Min".to_string(),
            all_zero_or_prio_min_strategy,
        ),
        ("All Zero/Big Min".to_string(), all_zero_or_big_min_strategy),
        (
            "All Big Zero/One Zero/Big Min".to_string(),
            all_big_zero_or_one_zero_or_big_min_strategy,
        ),
    ];

    println!("Simulating {} games for each strategy...", num_simulations);

    // Type alias for the result type to reduce complexity
    type SimulationResult = (f64, u8, u64, u8, std::time::Duration);

    let mut results = HashMap::new();

    for (name, strategy) in strategies {
        let start = Instant::now();
        let (avg_points, min_points, gravies, max_points) =
            run_simulations(strategy, num_simulations);
        let duration = start.elapsed();

        results.insert(
            name,
            (avg_points, min_points, gravies, max_points, duration),
        );
    }

    // Print results in a nicely formatted table
    println!(
        "\n{:<30} {:<10} {:>4} {:>8} {:>4} {:>10}",
        "Strategy", "Avg Points", "Min", "Gravies", "Max", "Time"
    );
    println!("{:-<72}", "");

    // Sort and display results
    let mut sorted_results: Vec<(&String, &SimulationResult)> = results.iter().collect();
    sorted_results.sort_by(|a, b| a.1.0.partial_cmp(&b.1.0).unwrap());

    for (name, (avg, min, gravies, max, duration)) in sorted_results {
        println!(
            "{:<30} {:>10.2} {:>4} {:>8} {:>4} {:>10.2?}",
            name, avg, min, gravies, max, duration
        );
    }
}
