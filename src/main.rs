use rand::Rng;
use rand::SeedableRng;
use rand::rngs::SmallRng;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::time::Instant;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Faces {
    Six,
    Eight,
    Ten,
    Twelve,
}

impl Faces {
    fn value(&self) -> u8 {
        match self {
            Faces::Six => 6,
            Faces::Eight => 8,
            Faces::Ten => 10,
            Faces::Twelve => 12,
        }
    }
}

#[derive(Debug, Clone)]
struct Die {
    faces: Faces,
    points: u8,
}

impl Die {
    fn six() -> Self {
        Die {
            faces: Faces::Six,
            points: 6,
        }
    }

    fn eight() -> Self {
        Die {
            faces: Faces::Eight,
            points: 8,
        }
    }

    fn ten() -> Self {
        Die {
            faces: Faces::Ten,
            points: 10,
        }
    }

    fn twelve() -> Self {
        Die {
            faces: Faces::Twelve,
            points: 12,
        }
    }

    #[cfg(test)]
    fn with_points(mut self, points: u8) -> Self {
        self.points = points;
        self
    }

    fn roll(&mut self, rng: &mut impl Rng) {
        self.points = rng.random_range(0..self.faces.value());
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
        let mut dice = Vec::with_capacity(15);

        // Add the 12 six-sided dice
        for _ in 0..12 {
            dice.push(Die::six());
        }

        // Add the special dice
        dice.push(Die::eight());
        dice.push(Die::ten());
        dice.push(Die::twelve());

        Game { dice }
    }

    fn roll_all(&mut self, rng: &mut impl Rng) {
        for die in &mut self.dice {
            die.roll(rng);
        }
    }

    fn remove_dice(&mut self, indices: &mut [usize]) -> u8 {
        indices.sort_unstable_by(|a, b| b.cmp(a));

        let mut points = 0;
        for index in indices {
            points += self.dice.swap_remove(*index).points();
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
            write!(f, "{} ", die.faces.value())?;
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
        .filter(|(_, die)| die.points() == 0 && die.faces != Faces::Six)
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
        .min_by(|(_, a), (_, b)| {
            let point_cmp = a.points().cmp(&b.points());
            match point_cmp {
                Ordering::Equal => b.faces.cmp(&a.faces),
                _ => point_cmp,
            }
        })
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

fn prio_min_for(die: &Die) -> i8 {
    die.faces.value() as i8 - 4 * die.points() as i8
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
    let mut best_max = &Faces::Six;
    let mut best_score = i8::MIN;

    for (i, die) in dice.iter().enumerate() {
        // Score function: higher is better - prioritize high max_value and low points
        let score = prio_min_for(die);
        if score > best_score || (score == best_score && die.faces > *best_max) {
            best_score = score;
            best_max = &die.faces;
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
        let big_dice_count = dice.iter().filter(|die| die.faces != Faces::Six).count();
        let big_zero_count = big_zeros.len();

        if big_dice_count == big_zero_count {
            return find_zero_point_dice(dice);
        }
        return big_zeros;
    }

    let all_zeros = find_zero_point_dice(dice);
    if !all_zeros.is_empty() {
        if dice.iter().filter(|die| die.faces != Faces::Six).count() == 0 {
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
            Die::ten().with_points(1),
        ];

        let zero_indices = find_zero_point_dice(&dice);
        assert_eq!(zero_indices, vec![0, 2]); // Only indices 0 and 2 have zero points
    }

    #[test]
    fn test_find_min_points_die() {
        let dice = vec![
            Die::six().with_points(3),
            Die::eight().with_points(1),
            Die::ten().with_points(4),
            Die::twelve().with_points(2),
        ];

        let min_index = find_min_points_die(&dice);
        assert_eq!(min_index, 1);
    }

    #[test]
    fn test_find_big_min_die() {
        let mut dice = vec![Die::six().with_points(1), Die::ten().with_points(1)];

        let min_index = find_big_min_die(&dice);
        assert_eq!(1, min_index);

        dice.reverse();

        let min_index = find_big_min_die(&dice);
        assert_eq!(0, min_index);
    }

    #[test]
    fn test_all_zero_or_prio_min_strategy() {
        let mut dice = vec![
            Die::six().with_points(3),
            Die::eight().with_points(1),
            Die::ten().with_points(4),
            Die::twelve().with_points(2),
        ];

        let prio_min = all_zero_or_prio_min_strategy(&dice);
        assert_eq!(vec![3], prio_min); // 12 - 4 * 2 = 4

        dice.remove(prio_min[0]);
        let prio_min = all_zero_or_prio_min_strategy(&dice);
        assert_eq!(vec![1], prio_min); // 8 - 4 * 1 = 4

        dice.remove(prio_min[0]);
        let prio_min = all_zero_or_prio_min_strategy(&dice);
        assert_eq!(vec![1], prio_min); // 10 - 4 * 4 = -6

        dice.remove(prio_min[0]);
        let prio_min = all_zero_or_prio_min_strategy(&dice);
        assert_eq!(vec![0], prio_min); // 6 - 4 * 3 = -6
    }

    #[test]
    fn test_all_zero_or_prio_min_strategy_hmm() {
        let mut dice = vec![Die::six().with_points(1), Die::twelve().with_points(3)];

        let prio_min = all_zero_or_prio_min_strategy(&dice);
        assert_eq!(vec![0], prio_min); // 6 - 4 * 1 = 2

        dice.remove(prio_min[0]);
        let prio_min = all_zero_or_prio_min_strategy(&dice);
        assert_eq!(vec![0], prio_min); // 12 - 4 * 3 = 0
    }

    #[test]
    fn test_all_zero_or_prio_min_deterministic() {
        let mut dice = vec![Die::eight().with_points(1), Die::twelve().with_points(2)];
        // 8 - 4 * 1 = 4
        // 12 - 4 * 2 = 4
        dice.iter().for_each(|d| assert_eq!(4, prio_min_for(d)));

        let prio_min = all_zero_or_prio_min_strategy(&dice);
        assert_eq!(12, dice[prio_min[0]].faces.value());

        dice.reverse();

        let prio_min = all_zero_or_prio_min_strategy(&dice);
        assert_eq!(12, dice[prio_min[0]].faces.value());
    }

    #[test]
    fn test_game_remove_dice() {
        let mut game = Game::new();
        game.dice = vec![
            Die::six().with_points(3),
            Die::eight().with_points(1),
            Die::ten().with_points(5),
            Die::twelve().with_points(2),
        ];

        let points = game.remove_dice(&mut [1, 3]);
        assert_eq!(points, 3); // 1 + 2 = 3 points
        assert_eq!(game.dice.len(), 2); // Should have 2 dice left
        assert_eq!(game.dice[0].points, 3); // First die should remain
        assert_eq!(game.dice[1].points, 5); // Third die should remain
    }

    #[test]
    fn test_full_game_simulation() {
        let points = simulate_game(all_zero_or_big_min_strategy, 800);
        assert_eq!(0, points);
    }
}

fn simulate_game(strategy: Strategy, seed: u64) -> u8 {
    let mut rng = SmallRng::seed_from_u64(seed);
    let mut game = Game::new();
    let mut total_points = 0;

    while !game.is_over() {
        game.roll_all(&mut rng);
        // if seed == 800 {
        //     println!("{}\n", game);
        // }
        let mut indices = strategy(&game.dice);
        total_points += game.remove_dice(&mut indices);
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
            // println!("seed: {}", i);
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
