Sudoku Solver
======
It will solve classic 9x9 sudoku puzzles using a hybrid approach of constraint propogation and recursive guessing. This was a project for learning Rust more than anything so the constraints are non exhaustive and the recursion could be more efficient. That said it is tested to work on all puzzles with a resonable solution space.

## Usage

```bash
# Solve a sudoku puzzle passed as a string and print each of the solve to the terminal
# the function that prints each step will use linux terminal escape codes to mark initial
# values of the puzzle, newly discovered values, and guessed values with a unique color
./sudoku_solver solve -v -s 002000063009000001006000400020180070900760000070490816000800007300040008008000940

# Run through the constraint propogation steps and return the next steps possible from the
# current board state. Will also print a detailed board with the calculated possible values
# for each unsolved cell. Note this mode will not use recursion.
./sudoku_solver hint -s 002000063009000001006000400020180070900760000070490816000800007300040008008000940

# Read a puzzle from a file
./sudoku_solver solve -v -i test.txt

# Acceptable file formats are:
#
# 002000063009000001006000400020180070900760000070490816000800007300040008008000940
#
# 002000063
# 009000001
# 006000400
# 020180070
# 900760000
# 070490816
# 000800007
# 300040008
# 008000940
#
# 002|000|063
# 009|000|001
# 006|000|400
# ---+---+---
# 020|180|070
# 900|760|000
# 070|490|816
# ---+---+---
# 000|800|007
# 300|040|008
# 008|000|940
```