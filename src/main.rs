use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use rand::Rng;
use std::collections::HashMap;
use std::process;
use statrs::distribution::ChiSquared;
use statrs::distribution::ContinuousCDF;
use std::io::Write;

struct Trait {
    name: String,
    frequency: f64,
}

fn read_csv(file_path: &str) -> Vec<Trait> {
    let file = File::open(file_path).expect("Failed to open file");
    let reader = BufReader::new(file);

    let mut traits = Vec::new();
    for line in reader.lines().skip(1) {
        let line = line.expect("Failed to read line");
        let fields: Vec<&str> = line.split(',').collect();

        let name = fields[0].to_string();
        let frequency = match fields[2].parse() {
            Ok(value) => {
                if value >= 0.0 && value <= 1.0 {
                    value
                } else {
                    eprintln!("Invalid frequency value for trait '{}'. Skipping.", name);
                    continue;
                }
            }
            Err(_) => {
                eprintln!("Invalid frequency value for trait '{}'. Skipping.", name);
                continue;
            }
        };

        traits.push(Trait {
            name,
            frequency,
        });
    }

    traits
}

fn copula_sampling(conditional_prob_matrix: &[[f64; 34]; 34], reference_traits: &[Trait], rng: &mut impl Rng, mode: &str) -> Vec<String> {
    let mut observation_strengths = Vec::new();
    let mut remaining_traits: Vec<usize> = (0..34).collect();

    let num_traits = match mode {
        "top5" => 5,
        "top10" => 10,
        _ => panic!("Invalid mode: {}", mode),
    };

    while observation_strengths.len() < num_traits && !remaining_traits.is_empty() {
        let index = rng.gen_range(0..remaining_traits.len());
        let trait_index = remaining_traits.remove(index);

        let mut conditional_probs = Vec::new();
        for &strength_index in &observation_strengths {
            let strength_index = strength_index as usize;
            let trait_index = trait_index as usize;
            conditional_probs.push(conditional_prob_matrix[strength_index][trait_index]);
        }

        let prob = conditional_probs.iter().fold(1.0, |acc, &p| acc * p);
        if rng.gen_bool(prob) {
            observation_strengths.push(trait_index);
        }
    }

    observation_strengths
        .iter()
        .map(|&index| reference_traits[index].name.clone())
        .collect()
}

fn simulate_with_priors(
    conditional_prob_matrix: &[[f64; 34]; 34],
    reference_traits: &[Trait],
    group_traits: &[Trait],
    group_size: usize,
    num_simulations: usize,
    verbose: bool,
    mode: &str,
) -> (f64, HashMap<String, f64>) {
    let mut rng = rand::thread_rng();
    let mut exact_match_count = 0;
    let mut trait_match_counts: HashMap<String, usize> = group_traits.iter().map(|t| (t.name.clone(), 0)).collect();

    let progress_bar_width = 50;
    let progress_interval = num_simulations / progress_bar_width;

    for simulation_index in 0..num_simulations {
        let mut simulated_group_traits = vec![0.0; group_traits.len()];

        for _ in 0..group_size {
            let observation_strengths = copula_sampling(conditional_prob_matrix, reference_traits, &mut rng, mode);

            for (i, group_trait) in group_traits.iter().enumerate() {
                if observation_strengths.contains(&group_trait.name) {
                    simulated_group_traits[i] += 1.0 / group_size as f64;
                }
            }
        }

        let rounded_simulated_group_traits: Vec<f64> = simulated_group_traits
            .iter()
            .map(|freq| (freq * 100.0).round() / 100.0)
            .collect();

        let is_exact_match = rounded_simulated_group_traits
            .iter()
            .zip(group_traits.iter())
            .all(|(simulated_freq, group_trait)| (*simulated_freq - group_trait.frequency).abs() < 1e-6);

        if is_exact_match {
            exact_match_count += 1;
        }

        for (i, group_trait) in group_traits.iter().enumerate() {
            if (rounded_simulated_group_traits[i] - group_trait.frequency).abs() < 1e-6 {
                *trait_match_counts.get_mut(&group_trait.name).unwrap() += 1;
            }
        }

        if verbose {
            println!("Simulation {}:", simulation_index + 1);
            for (trait_name, simulated_freq) in group_traits.iter().zip(rounded_simulated_group_traits.iter()) {
                println!("\t{}: {:.2}", trait_name.name, simulated_freq);
            }
            println!();
        }

        if (simulation_index + 1) % progress_interval == 0 {
            let progress = (simulation_index + 1) as f64 / num_simulations as f64;
            let filled_width = (progress * progress_bar_width as f64) as usize;
            let empty_width = progress_bar_width - filled_width;
            print!("\rProgress: [{}{}] {:.2}%", "=".repeat(filled_width), " ".repeat(empty_width), progress * 100.0);
            std::io::stdout().flush().unwrap();
        }
    }
    println!();

    let exact_match_probability = exact_match_count as f64 / num_simulations as f64;
    let trait_probabilities: HashMap<String, f64> = trait_match_counts
        .into_iter()
        .map(|(name, count)| (name, count as f64 / num_simulations as f64))
        .collect();

    (exact_match_probability, trait_probabilities)
}

fn read_probability_matrix(file_path: &str) -> [[f64; 34]; 34] {
    let file = File::open(file_path).expect("Failed to open file");
    let reader = BufReader::new(file);

    let mut matrix = [[0.0; 34]; 34];

    for (i, line) in reader.lines().enumerate() {
        if i == 0 {
            continue; // Skip the header row
        }

        let line = line.expect("Failed to read line");
        let values: Vec<f64> = line
            .split(',')
            .skip(1) // Skip the first column (trait name)
            .map(|value| value.parse().unwrap_or(0.0))
            .collect();

        for (j, &value) in values.iter().enumerate() {
            matrix[i - 1][j] = value;
        }
    }

    matrix
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 7 {
        eprintln!("Usage: cargo run -- <reference_data.csv> <group_data.csv> <group_size> <num_simulations> <verbose> <mode>");
        process::exit(1);
    }

    let reference_data_file = &args[1];
    let group_data_file = &args[2];
    let group_size: usize = args[3].parse().expect("Invalid group size");
    let num_simulations: usize = args[4].parse().expect("Invalid number of simulations");
    let verbose: bool = args[5].parse().expect("Invalid verbose flag");
    let mode = &args[6];

    if mode != "top5" && mode != "top10" {
        eprintln!("Invalid mode: {}. Mode should be either 'top5' or 'top10'.", mode);
        process::exit(1);
    }

    let reference_traits = read_csv(reference_data_file);
    let group_traits = read_csv(group_data_file);

    if reference_traits.len() != 34 {
        eprintln!("Error: reference_data.csv must contain exactly 34 traits.");
        process::exit(1);
    }

    if group_traits.len() != 34 {
        eprintln!("Error: group_data.csv must contain exactly 34 traits.");
        process::exit(1);
    }

    let probability_matrix = read_probability_matrix("probability_matrix.csv");

    let (exact_match_probability, trait_probabilities) = 
        simulate_with_priors(&probability_matrix, &reference_traits, &group_traits, group_size, num_simulations, verbose, mode);
    println!("Probability of observing exactly the group_data: {}", exact_match_probability);
    println!("\nTrait probabilities:");

    let mut sorted_trait_probabilities: Vec<(String, f64)> = trait_probabilities.into_iter().collect();
    sorted_trait_probabilities.sort_by(|a, b| a.0.cmp(&b.0));

    for (trait_name, probability) in &sorted_trait_probabilities {
        println!("\t{}: {}", trait_name, probability);
    }

    let significance_level = 0.05;
    let num_traits = 34;
    let bonferroni_significance_level = significance_level / num_traits as f64;
    let degrees_of_freedom = num_traits as f64 - 1.0;

    let critical_value = ChiSquared::new(degrees_of_freedom).unwrap().inverse_cdf(1.0 - bonferroni_significance_level);

    let total_observations = group_size as f64 * num_simulations as f64;

    println!("\nStatistically significant traits (Bonferroni-corrected significance level = {}):", bonferroni_significance_level);
    for (trait_name, observed_prob) in &sorted_trait_probabilities {
        let expected_prob = reference_traits.iter().find(|t| &t.name == trait_name).unwrap().frequency;
        let observed_count = observed_prob * total_observations;
        let expected_count = expected_prob * total_observations;
        let chi_square = (observed_count - expected_count).powi(2) / expected_count;

        if chi_square > critical_value {
            println!("\t{}: observed = {}, expected = {}, chi-square = {}", trait_name, observed_prob, expected_prob, chi_square);
        }
    }
}