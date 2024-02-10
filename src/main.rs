// Hide warnings for unused stuff while developing
#![allow(dead_code)]
#![allow(unused)]

use core::panic;
use std::{borrow::Borrow, cell::{Cell, RefCell}, collections::HashSet, fs::{self, File}, io::{Chain, Read}, ops::{Deref, Index}, path::PathBuf, usize};
use clap::{ Parser, Subcommand};

//-----------------------------------------------------------------------------
// Structs and Enums
//-----------------------------------------------------------------------------

//-------------------------------------
// CLI Parsing
//-------------------------------------

#[derive(Parser)]
#[command(author, version, about, long_about = None)] // Read from `Cargo.toml`
struct Cli {
    /// The operation to run
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// does testing things
    Test,

    /// only runs a single pass then returns the found next step(s)
    Hint{
        /// Read a puzzle from the command line as 81 numeric digits with '0' representing unknown values
        #[arg(short = 's', long, value_name = "STRING")]
        in_string: Option<String>,

        /// Read a puzzle from a text file
        #[arg(short = 'i', long, value_name = "FILE")]
        in_file: Option<PathBuf>,

        /// Verbose mode. Will write each step of the solve to the terminal
        #[arg(short, long)]
        verbose: bool,
    },
    
    /// solves the puzzle
    Solve {

        /// Read a puzzle from the command line as 81 numeric digits with '0' representing unknown values
        #[arg(short = 's', long, value_name = "STRING")]
        in_string: Option<String>,

        /// Read a puzzle from a text file
        #[arg(short = 'i', long, value_name = "FILE")]
        in_file: Option<PathBuf>,

        /// Verbose mode. Will write each step of the solve to the terminal
        #[arg(short, long)]
        verbose: bool,
    },
}


//-------------------------------------
// Game Cells
//   (are individual board elements)
//-------------------------------------
#[derive(Clone,Copy,Debug,PartialEq)]
enum CellState {
    Initial, // initial cells
    Solved, //cells that have been solved in a previous itteration of the solver loop
    New, //cells that have just been solved
    Unsolved, //Default for unsolved cells
    Guess, //used in recursion
}
#[derive(Clone,Debug)]
struct GameCell {
    pub value:usize,
    pub possible_values:Vec<usize>,
    pub state:CellState,
}
impl GameCell {
    /// A new uninitialized cell set to a value of zero and 1..=9 possible values
    fn new() -> GameCell {
        GameCell{value:0,possible_values:vec![1,2,3,4,5,6,7,8,9],state:CellState::Unsolved}
    }

    /// Removes a value, if present, from the possible value list
    fn remove_possible_cell_value(&mut self, v:usize) {
        if self.value == 0 {
            self.possible_values.retain(|&x| x != v);
        }
    }

    /// Keeps a particular possible value and removes all others
    fn keep_only_possible_cell_value(&mut self, v:usize) {
        if self.value == 0 {
            self.possible_values.retain(|&x| x == v);
        }
    }

    /// Keeps a particular possible value pair and removes all others
    fn keep_only_possible_cell_value_pair(&mut self, v:(usize,usize)) {
        if self.value == 0 {
            self.possible_values.retain(|&x| x == v.0 || x == v.1);
        }
    }
    
    /// Sets the value of the cell and removes the list of possible values, unless setting to zero, in which case it will reset the possible values
    fn set_value(&mut self, v:usize) {
        self.value=v;
        if v == 0 {
            self.possible_values = vec![1,2,3,4,5,6,7,8,9];
        } else {
            self.possible_values.clear();
        }
    }

    /// Checks if there is only 1 possible value and updates the cells value accordingly.
    fn check_possible(&mut self) -> bool {
        if self.possible_values.len() == 1 {
            self.set_value(self.possible_values[0]);
            self.set_newly_solved();
            return true;
        }
        false
    }

    /// Prints the value of the cell
    fn print(&self) {print!("{}",self.value);}

    /// Prints the value of the cell using linux terminal escape sequences to color the output according to if the cell was an initial value of the puzzle, just solved, or previously solved
    fn print_color(&self) {
        match self.state {
            CellState::Initial => print!("\x1b[91m\x1b[1m{}\x1b[0m",self.value),
            CellState::Solved => print!("{}",self.value),
            CellState::New => print!("\x1b[97m\x1b[1m{}\x1b[0m",self.value),
            CellState::Unsolved => print!(" "),
            CellState::Guess => print!("\x1b[95m\x1b[1m{}\x1b[0m",self.value),
        }
    }

    /// Prints cell in a 9 character wide format listing either the cell value or the possible cell values
    fn print_detailed(&self) {
        if self.value != 0 {
            print!("    {}    ",self.value);
        } else {
            for i in 1..=9 {
                if self.possible_values.contains(&i) {
                    print!("{}",&i);
                } else {
                    print!(" ");
                }
            }
        }
    }

    /// Initializes the cell's internal state (Used for cell colors in terminal output)
    fn set_initial(&mut self) {if self.value == 0 {self.state = CellState::Unsolved;}else{self.state = CellState::Initial;}}

    /// Sets the cell to solved (Used for cell colors in terminal output)
    fn set_newly_solved(&mut self) {self.state = CellState::New;}

    /// Sets the cell to previously solved (Used for cell colors in terminal output)
    fn set_previously_solved(&mut self) {if self.state == CellState::New {self.state = CellState::Solved;} }

    fn set_guessed(&mut self) {self.state = CellState::Guess;}
}

//-------------------------------------
// Game Board
//   Collection of Game Cells in 2d grid
//-------------------------------------
#[derive(Debug)]
struct GameBoard {
    pub board: Vec< Vec< RefCell<GameCell> > >,
}
impl GameBoard {

    fn new() -> GameBoard {
        let mut b = vec![
            vec![RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new())],
            vec![RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new())],
            vec![RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new())],
            vec![RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new())],
            vec![RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new())],
            vec![RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new())],
            vec![RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new())],
            vec![RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new())],
            vec![RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new()),RefCell::new(GameCell::new())]
        ];
        GameBoard{
            board:b
        }
    }

    /// testing function to inject a board with a known solution
    fn init_board_with_test_values(&mut self) {
        self.init_board_from_string("091000203000002700705600000000713060009000000000500002000007304000060009000300015".to_string()); //solves in 8 steps with distribution
    }

    /// removes any cell possible values if that value already exists in it's row
    fn set_possible_values_by_row(&mut self) { //will loop through rows to remove possible values
        //check rows
        for i in 0..9 {
            for j in 0..9 { //for each cell in the row
                let this_value = self.board[i][j].borrow().deref().value;
                if this_value != 0 {
                    for k in 0..9 {
                        //remove value from row
                        self.board[i][k].get_mut().remove_possible_cell_value(this_value);
                    }
                }
            }
        }
    }

    /// removes any cell possible values if that value already exists in it's column
    fn set_possible_values_by_col(&mut self) { //will loop through columns to remove possible values
        //check columns
        for i in 0..9 {
            for j in 0..9 { //for each cell in the col
                let this_value = self.board[j][i].borrow().deref().value;
                if this_value != 0 {
                    for k in 0..9 {
                        //remove value from col
                        self.board[k][i].get_mut().remove_possible_cell_value(this_value);
                    }
                }
            }
        }
    }

    /// removes any cell possible values if that value already exists in its square
    fn set_possible_values_by_square(&mut self) {
        for sq_index      in [(0,0),(0,3),(0,6),(3,0),(3,3),(3,6),(6,0),(6,3),(6,6)]{ //2d vector offsets for the top left most square of each 3x3 cell
            for sq_offset in [(0,0),(0,1),(0,2),(1,0),(1,1),(1,2),(2,0),(2,1),(2,2)]{ //2d offset values for each other square in the 3x3 cell
                let this_value = self.board[sq_index.0+sq_offset.0][sq_index.1+sq_offset.1].borrow().deref().value;
                for sq_offset_2 in [(0,0),(0,1),(0,2),(1,0),(1,1),(1,2),(2,0),(2,1),(2,2)]{
                    self.board[sq_index.0+sq_offset_2.0][sq_index.1+sq_offset_2.1].get_mut().remove_possible_cell_value(this_value);
                }
            }
        }
    }

    /// prints the board without color
    fn print_board(&self) {
        for i in 0..9 {
            for j in 0..9 {
                self.board[i][j].borrow().print();
                if j == 2 || j == 5 {
                    print!("|");
                }
            }
            println!();
            if i == 2 || i == 5 {
                println!{"---+---+---"};
            }
        }
        println!();
    }

    /// print the board using linux color escapes. Red for Initial cells and white&Bold for newly solved cells
    fn print_color_board(&self) {
        for i in 0..9 {
            for j in 0..9 {
                self.board[i][j].borrow().print_color();
                if j == 2 || j == 5 {
                    print!("|");
                }
            }
            println!();
            if i == 2 || i == 5 {
                println!{"---+---+---"};
            }
        }
        println!();
    }

    /// print all cells padded to 9 characters, if unsolved print the possible values remaining
    fn print_detailed_board(&self) {
        for i in 0..9 {
            for j in 0..9 {
                self.board[i][j].borrow().print_detailed();
                print!(".");
                if j == 2 || j == 5 {
                    print!("|");
                }
            }
            println!();
            if i == 2 || i == 5 {
                println!{"------------------------------+------------------------------+------------------------------"};
            }
        }
        println!();
    }

    /// checks all unsolved cells to see if there is only one possible value remaining and updates the cell to that value and marks solved. Boolean return value indicates changes were made
    fn set_values_from_possible(&mut self) -> bool {
        let mut changes_made = false;
        for i in 0..9 {
            for j in 0..9 { //for each cell in the row
                let mut tmp_bool = self.board[i][j].get_mut().check_possible();
                if tmp_bool {
                    changes_made = true;
                }
            }
        }
        changes_made
    }

    /// Runs every loop of the algorithm to set the newly solved cells to just solved. Used for coloring newly solved cells differently
    fn set_previously_solved_cells(&mut self) {
        for i in 0..9 {
            for j in 0..9 {
                self.board[i][j].get_mut().set_previously_solved();
            }
        }
    }

    /// Used to declare the current state of the board the initial state. All solved cells will be colored accordingly
    fn set_initial_cells(&mut self) {
        for i in 0..9 {
            for j in 0..9 {
                self.board[i][j].get_mut().set_initial();
            }
        }
    }

    /// takes an 81 character string and initialized the board with the appropriate values. 0 indicates unsolved
    fn init_board_from_string(&mut self,in_str:String) {
        let value_vector = convert_string_to_vector(&in_str);
        for i in 0..9 {
            for j in 0..9 {
                self.board[i][j].get_mut().set_value( value_vector[ (i*9)+j ] );
            }
        }
        self.set_initial_cells();
    }

    fn init_board_from_file(&mut self,in_file:PathBuf) {
        let mut f = File::open( in_file );
        if f.is_err() {
            panic!("Error opening file");
        }
        let mut contents = String::new();
        let mut o = f.unwrap();
        o.read_to_string(&mut contents);
        let mut t: String = contents.chars().filter(|c| c.is_numeric()).collect();
        t.retain(|x|x != '\n');
        t.retain(|x|x != '-');
        t.retain(|x|x != '|');
        t.retain(|x|x != '+');
        t.retain(|x|x != ' ');
        t.retain(|x|x != '*');
        t.retain(|x|x != '_');
        t.retain(|x|x != '.');
        if t.len() == 81 {
            self.init_board_from_string(t);
        } else {
            panic!("Invalid file contents:\n{}",t);
        }
    }
    /// checks rows, columns, and squares to see if there are any possible values that appear only once
    fn set_possible_values_by_distribution(&mut self) {
        //rows
        for i in 0..9 {
            let mut distribution_vector: Vec<usize> = vec![0,0,0,0,0,0,0,0,0];
            for j in 0..9 {
                if self.board[i][j].borrow().deref().value == 0 {
                    increment_values_by_index(&mut distribution_vector, 
                        &self.board[i][j].borrow().possible_values)
                }
            }
            //println!("{:?}",distribution_vector);
            let mut k :usize = 1;//used to track what value things are
            for x in distribution_vector.clone() {
                if x == 1 {
                    for j in 0..9 {
                        if self.board[i][j].borrow().deref().possible_values.contains(&k) {
                            self.board[i][j].get_mut().keep_only_possible_cell_value(k);
                        }
                    }
                }
                k += 1;
            }
        }
        //columns
        for i in 0..9 {
            let mut distribution_vector: Vec<usize> = vec![0,0,0,0,0,0,0,0,0];
            for j in 0..9 {
                if self.board[j][i].borrow().deref().value == 0 {
                    increment_values_by_index(&mut distribution_vector, 
                        &self.board[j][i].borrow().possible_values)
                }
            }
            //println!("{:?}",distribution_vector);
            let mut k :usize = 1;//used to track what value things are
            for x in distribution_vector.clone() {
                if x == 1 {
                    for j in 0..9 {
                        if self.board[j][i].borrow().deref().possible_values.contains(&k) {
                            self.board[j][i].get_mut().keep_only_possible_cell_value(k);
                        }
                    }
                }
                k += 1;
            }
        }
        //squares
        for sq_index      in [(0,0),(0,3),(0,6),(3,0),(3,3),(3,6),(6,0),(6,3),(6,6)]{ //2d vector offsets for the top left most square of each 3x3 cell
            let mut distribution_vector: Vec<usize> = vec![0,0,0,0,0,0,0,0,0];
            for sq_offset in [(0,0),(0,1),(0,2),(1,0),(1,1),(1,2),(2,0),(2,1),(2,2)]{ //2d offset values for each other square in the 3x3 cell
                if self.board[sq_index.0+sq_offset.0][sq_index.1+sq_offset.1].borrow().deref().value == 0 {
                    increment_values_by_index(&mut distribution_vector, 
                        &self.board[sq_index.0+sq_offset.0][sq_index.1+sq_offset.1].borrow().possible_values)
                }

            }
            //println!("{:?}",distribution_vector);
            let mut k :usize = 1;//used to track what value things are
            for x in distribution_vector.clone() {
                if x == 1 {
                    for sq_offset in [(0,0),(0,1),(0,2),(1,0),(1,1),(1,2),(2,0),(2,1),(2,2)]{
                        if self.board[sq_index.0+sq_offset.0][sq_index.1+sq_offset.1].borrow().deref().possible_values.contains(&k) {
                            self.board[sq_index.0+sq_offset.0][sq_index.1+sq_offset.1].get_mut().keep_only_possible_cell_value(k);
                        }
                    }
                }
                k += 1;
            }
        }
    }

    /// check the rows and columns in a square to see if the sum of known values and possible values == 3 and remove those values along the row/col and square
    fn set_possible_values_by_short_segments(&mut self) {
        //rows
        for i in 0..9 {
            for (j,range,not_range) in [(0,3..9,0..3),(3,0..3,3..6),(3,6..9,3..6),(6,0..6,6..9)]{
                let mut segment: Vec<usize> = get_domain(
                    self.board[i][j  ].borrow().deref(),
                    self.board[i][j+1].borrow().deref(),
                    self.board[i][j+2].borrow().deref() );
                if segment.len() == 3 {
                    for val in segment {
                        for k in range.clone() {
                            self.board[i][k].get_mut().remove_possible_cell_value(val);
                        }
                        //this logic is a bit ugly but what it does is check the other two parallel line segments in the parent square
                        match i % 3 {
                            0 => {
                                for k in not_range.clone() {
                                    self.board[i+1][k].get_mut().remove_possible_cell_value(val);
                                    self.board[i+2][k].get_mut().remove_possible_cell_value(val);
                                }
                            }
                            1 => {
                                for k in not_range.clone() {
                                    self.board[i-1][k].get_mut().remove_possible_cell_value(val);
                                    self.board[i+1][k].get_mut().remove_possible_cell_value(val);
                                }
                            }
                            2 => {
                                for k in not_range.clone() {
                                    self.board[i-2][k].get_mut().remove_possible_cell_value(val);
                                    self.board[i-1][k].get_mut().remove_possible_cell_value(val);
                                }
                            }
                            _ => panic!("math is hard for the compiler")
                        }
                    }
                }
            }
        }
        //cols
        for i in 0..9 {
            for (j,range,not_range) in [(0,3..9,0..3),(3,0..3,3..6),(3,6..9,3..6),(6,0..6,6..9)]{
                let mut segment: Vec<usize> = get_domain(
                    self.board[j  ][i].borrow().deref(),
                    self.board[j+1][i].borrow().deref(),
                    self.board[j+2][i].borrow().deref() );
                if segment.len() == 3 {
                    for val in segment {
                        for k in range.clone() {
                            self.board[k][i].get_mut().remove_possible_cell_value(val);
                        }
                        //this logic is a bit ugly but what it does is check the other two parallel line segments in the parent square
                        match i % 3 {
                            0 => {
                                for k in not_range.clone() {
                                    self.board[k][i+1].get_mut().remove_possible_cell_value(val);
                                    self.board[k][i+2].get_mut().remove_possible_cell_value(val);
                                }
                            }
                            1 => {
                                for k in not_range.clone() {
                                    self.board[k][i-1].get_mut().remove_possible_cell_value(val);
                                    self.board[k][i+1].get_mut().remove_possible_cell_value(val);
                                }
                            }
                            2 => {
                                for k in not_range.clone() {
                                    self.board[k][i-2].get_mut().remove_possible_cell_value(val);
                                    self.board[k][i-1].get_mut().remove_possible_cell_value(val);
                                }
                            }
                            _ => panic!("math is hard for the compiler")
                        }
                    }
                }
            }
        }
    }

    /// function to check if there are no unsolved cells remaining
    fn is_solved(&self)->bool{
        for i in 0..9 {
            for j in 0..9{
                if self.board[i][j].borrow().deref().value == 0 {
                    return false;
                }
            }
        }
        return true;
    }

    /// runs through the constraint propagation algorithm once and returns the result
    fn hint(&mut self) {
        self.set_possible_values_by_row();
        self.set_possible_values_by_col();
        self.set_possible_values_by_square();
        self.set_possible_values_by_distribution();
        self.set_possible_values_by_short_segments();
    
        self.set_values_from_possible();
        self.print_color_board();
        println!();
        self.print_detailed_board();
    }

    /// Will return the current game board as a 81 character string with '0' representing unsolved values.
    fn board_to_string (&self)->String {
        let mut returned :Vec<usize> = Vec::new();
        returned.resize(81, 0);
        for i in 0..9 {
            for j in 0..9 {
                returned[(i*9)+j] = self.board[i][j].borrow().deref().value
            }
        }
        returned.into_iter().map(|i| i.to_string()).collect::<String>()
    }

    fn clone (&self)->GameBoard{
        let mut returned = GameBoard::new();
        returned.board = self.board.clone();
        returned
    }

    /// Will return the row and column of the first unsolved gamecell with the fewest possible values. will return none if used on a solved board.
    fn get_smallest_possible_gamecell_by_idx (&self) -> Option<(usize,usize)> {
        let mut smallest = 0;
        for i in 0..9 {
            for j in 0..9 {
                if smallest !=0 {
                    let tmp = self.board[i][j].borrow().deref().possible_values.len();
                    if tmp != 0 && tmp < smallest {smallest=tmp;}
                } else {
                    smallest = self.board[i][j].borrow().deref().possible_values.len();
                }
                
            }
        }
        if smallest == 0 {return None;}
        for i in 0..9 {
            for j in 0..9 {
                if smallest == self.board[i][j].borrow().deref().possible_values.len() {
                    return Some((i,j));
                }
                
            }
        }
        return None;
    }

    /// check to see if there are any cells that are not assigned and have no potential values
    fn is_unsolvable (&self) -> bool{
        for i in 0..9 {
            for j in 0..9 {
                if self.board[i][j].borrow().possible_values.len() == 0 && self.board[i][j].borrow().deref().value == 0 {
                    return true;
                }
            }
        }
        false
    }
    
    /// primary solve loop. Will loopt through using constraint propogation until the board is solved or until there are no moves left. It will then create 
    fn solve_loop(&mut self,verbose:bool) -> bool {
        loop {
            self.set_possible_values_by_row();
            self.set_possible_values_by_col();
            self.set_possible_values_by_square();
            self.set_possible_values_by_distribution();
            self.set_possible_values_by_short_segments();

            let mut updated = self.set_values_from_possible();
            if !updated && !self.is_unsolvable() {
                // recursion logic

                // debug stuff
                //println!("{}",self.board_to_string());
                
                let target = self.get_smallest_possible_gamecell_by_idx().unwrap();
                let target_size = self.board[target.0][target.1].borrow().possible_values.len();
                
                let mut possible_gameboards :Vec<GameBoard> = Vec::new();
                possible_gameboards.resize_with(target_size, ||{self.clone()});
                let possible_values_to_guess = self.board[target.0][target.1].borrow().possible_values.clone();
                for i in 0..target_size {
                    possible_gameboards[i].board[target.0][target.1].get_mut().set_value(possible_values_to_guess[i]);
                    possible_gameboards[i].board[target.0][target.1].get_mut().set_guessed();
                }
                for i in 0..target_size {
                    if possible_gameboards[i].solve_loop(false) {
                        //update logic
                        self.board[target.0][target.1].get_mut().set_value(possible_values_to_guess[i]);
                        self.board[target.0][target.1].get_mut().set_guessed();
                        updated = true;
                        break;
                    }
                    if i == target_size - 1 && updated == false {return false;}
                }
                //return false;
            }
            if verbose && updated {
                self.print_color_board();
            }
            self.set_previously_solved_cells();
            if self.is_solved() {return true;}
            if self.is_unsolvable() {return false;}
        }
    }
    
    /// Solve using the constraint propogation algorithm.
    fn solve(&mut self,verbose:bool) {
        if !self.solve_loop(verbose) {
            self.print_detailed_board();
        }
        if !verbose {
            self.print_board();
        }
    }

}


//-----------------------------------------------------------------------------
// Helper functions
//     (All written by chatgpt)
//-----------------------------------------------------------------------------
fn convert_string_to_vector(input: &str) -> Vec<usize> {
    if input.len() != 81 {
        panic!("Input string must be exactly 81 characters long");
    }

    let mut result = Vec::with_capacity(81);

    for c in input.chars() {
        match c {
            '0'..='9' => {
                let digit = c.to_digit(10).unwrap() as usize;
                result.push(digit);
            }
            _ => panic!("Invalid character found in input: {}", c),
        }
    }

    result
}

fn increment_values_by_index(base_vector: &mut Vec<usize>, indices_to_increment: & Vec<usize>) {
    for &index in indices_to_increment {
        let adjusted_index = index.checked_sub(1).unwrap_or_else(|| {
            panic!("Index {} is out of bounds for vector of length 9", index);
        });

        if adjusted_index < base_vector.len() {
            base_vector[adjusted_index] += 1;
        } else {
            panic!("Index {} is out of bounds for vector of length 9", index);
        }
    }
}

fn get_domain(cell1:&GameCell,cell2:&GameCell,cell3:&GameCell)->Vec<usize>{
    let mut returnvec: Vec<usize> = Vec::new();
    if cell1.value != 0 {returnvec.push(cell1.value);} else {returnvec.extend(cell1.possible_values.clone());}
    if cell2.value != 0 {returnvec.push(cell2.value);} else {returnvec.extend(cell2.possible_values.clone());}
    if cell3.value != 0 {returnvec.push(cell3.value);} else {returnvec.extend(cell3.possible_values.clone());}

    // remove duplicates
    let mut unique: HashSet<_> = returnvec.drain(..).collect();
    let returnvec: Vec<usize> = unique.into_iter().collect();
    return returnvec;
}


//-----------------------------------------------------------------------------
// Main
//-----------------------------------------------------------------------------
fn main() {

    let args = Cli::parse();
    let mut sudoku_board = GameBoard::new();

    match args.command.unwrap() {
        Commands::Test => {
            
            sudoku_board.init_board_with_test_values();
            sudoku_board.set_initial_cells();
            println!("Initial Board:");
            sudoku_board.print_board();
            println!();
        
            sudoku_board.set_possible_values_by_row();
            sudoku_board.set_possible_values_by_col();
            sudoku_board.set_possible_values_by_square();
            sudoku_board.set_possible_values_by_distribution();
        
            while sudoku_board.set_values_from_possible() {
                sudoku_board.set_possible_values_by_row();
                sudoku_board.set_possible_values_by_col();
                sudoku_board.set_possible_values_by_square();
                sudoku_board.set_possible_values_by_distribution();
                sudoku_board.print_color_board();
                sudoku_board.set_previously_solved_cells();
                println!();
            }
        
            println!();
            println!("After Algo:");
            sudoku_board.print_board();
        }
        Commands::Hint { in_string, in_file, verbose } => {
            if in_string.is_some() && in_file.is_some() {
                panic!("you can only supply one puzzle to solve at a time");
            } else if in_string.is_none() && in_file.is_none() {
                panic!("you must supply a puzzle to solve");
            } else if in_string.is_some() {
                sudoku_board.init_board_from_string(in_string.unwrap());
            } else if in_file.is_some() {
                sudoku_board.init_board_from_file(in_file.unwrap());
            }
            sudoku_board.set_initial_cells();
            sudoku_board.hint();

        }
        Commands::Solve { in_string, in_file, verbose } => {
            
            if in_string.is_some() && in_file.is_some() {
                panic!("you can only supply one puzzle to solve at a time");
            } else if in_string.is_none() && in_file.is_none() {
                panic!("you must supply a puzzle to solve");
            } else if in_string.is_some() {
                sudoku_board.init_board_from_string(in_string.unwrap());
            } else if in_file.is_some() {
                sudoku_board.init_board_from_file(in_file.unwrap());
            }
            sudoku_board.set_initial_cells();
            sudoku_board.solve(verbose);
        }
    }
}


