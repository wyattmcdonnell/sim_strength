### A simple simulator for Gallup/StrengthsFinder 2.0 data
`sim_strength` is a simple simulator for understanding how a dataset of Clifton StrengthsFinder group frequencies (`group_data.csv`) is distributed relative to observed strength frequencies observed in a reference sample of ~12M Americans circa 2018.

### Assumptions
- We assume that choosing a top 5 is not very different from choosing a top 10
  - Several traits tend to cluster (e.g. "Relationship Building", "Influencing", etc.)
  - Some traits are frequently observed with each other (Intellection, Input; 50.9%)
  - Some traits are rarely observed with each other (Ideation, Discipline; 0.5%)
- We assume that the conditional probabilities of traits being co-observed in a random sample of "top 5" results of 250,000 Americans generalize to 10

### Input files
`group_data.csv` and `reference_data.csv` should both have frequencies in the range `{0:1}`. Please note that the simulator rounds to the second digit (e.g. `0.375` becomes `0.38`). As there are `34` traits, both of these files should have entries for `34` traits. Note that you may wish to use your own `reference_data.csv` file from 2019 frequencies reported by Gallup for many countries. The `reference_data.csv` file in this repository refers to the American frequencies reported by Gallup.

`probability_matrix.csv` provides a conditional probability matrix of observing each pair of traits as reported in 250,000 random observations of American respondents in 2019.

### Usage
```
Usage: cargo run -- <reference_data.csv> <group_data.csv> <group_size> <num_simulations> <verbose> <mode>
```
The program will run `num_simulations` of size `group_size` and report observed simulated frequencies for each of the 34 traits, and will report some simple statistical testing as well. Note that providing `true` for the `verbose` flag will print the results of each simulation to `stdout`. Consider the following sample command:
```
cargo run --release reference_data.csv group_data.csv 10 10000 false top5
```
The above command will run `10000` simulations of a group of `10` individuals, choose 5 top traits/strengths for each observation in each simulation, print a progress bar, and then report results.

### Interpretation
Interpret at your own risk. Here be dragons! 🐉
