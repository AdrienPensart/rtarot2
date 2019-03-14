use failure::Error;
use std::fmt;
use rand::Rng;
use crate::deck::*;
use crate::mode::*;
use crate::contract::*;
use crate::player::*;
use crate::errors::*;
use crate::turn::*;
use crate::card::*;
use crate::role::*;
use crate::team::*;
use crate::helpers::*;

#[derive(Debug)]
pub struct Game {
    dog: Deck,
    deck: Deck,
    mode: Mode,
    players: Vec<Player>,
    random: bool,
    auto: bool,
    petit_au_bout: Option<Team>,
    defense_cards: usize,
    attack_cards: usize,
}

impl fmt::Display for Game {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Game with dog {} and players : ", self.dog)?;
        for p in &self.players {
            writeln!(f, "\t{}", p)?;
        }
        Ok(())
    }
}

impl<'a> Default for Game {
    fn default() -> Game
    {
        Game {
            random: false,
            auto: false,
            petit_au_bout: None,
            defense_cards: 0,
            attack_cards: 0,
            dog: Deck::default(),
            deck: Deck::build_deck(),
            players: vec![Player::new(Mode::default(), false); Mode::default() as usize],
            mode: Mode::default(),
        }
    }
}

impl Game
{
    pub fn new(mode: Mode, random: bool, auto: bool) -> Game {
        let mut game = Game {
            random,
            auto,
            players: vec![Player::new(mode, random); mode as usize],
            mode,
            ..Game::default()
        };
        for (i, p) in game.players.iter_mut().enumerate() {
            if let Ok(name) = default_name(i) {
                p.name = name;
            }
            p.mode = mode;
        }
        game
    }
    fn is_consistent(&self) {
        assert!(self.players.iter().map(|ref p| p.total).sum::<f64>() == 0.0)
    }
    pub fn distribute(&mut self) -> Result<(), Error> {
        self.deck.append(self.dog.give_all());
        for p in self.players.iter_mut() {
            self.deck.append(p.hand.give_all());
            self.deck.append(p.owned.give_all());
            p.prepare();
        }
        self.petit_au_bout = None;
        self.defense_cards = 0;
        self.attack_cards = 0;

        self.dog = self.deck.give(self.mode.dog_size());
        self.dog.sort();
        for p in self.players.iter_mut() {
            p.hand.append(self.deck.give(self.mode.cards_per_player()))
        }
        for p in self.players.iter_mut() {
            if p.hand.petit_sec() {
                // RULE: PetitSec cancel the game
                Err(TarotErrorKind::PetitSec)?
            }
            p.hand.sort();
        }
        Ok(())
    }
    pub fn bidding(&mut self) -> Result<(), Error> {
        let mut contracts = vec![
            Contract::Pass,
            Contract::Petite,
            Contract::Garde,
            Contract::GardeSans,
            Contract::GardeContre,
        ];

        let mut slam_index : Option<usize> = None;
        for (i, p) in self.players.iter_mut().enumerate() {
            if self.auto && contracts.len() == 1 {
                p.contract = Some(Contract::Pass);
                println!("Auto pass");
                continue
            }

            p.contract = if self.random {
                Some(contracts[rand::thread_rng().gen_range(0, contracts.len())])
            } else {
                loop {
                    println!("{} must play : {}", &p, &p.hand);
                    println!("Choose a contract, possibilities :");
                    for (i, c) in contracts.iter().enumerate() {
                        println!("\t{} : press {}", c, i);
                    }
                    let contract_index: Result<usize, _> = read_index();
                    match contract_index {
                        Ok(index) => {
                            if index < contracts.len() {
                                break Some(contracts[index])
                            } else {
                                println!("Error, please retry");
                            }
                        },
                        Err(_) => {
                            println!("Error, please retry")
                        }
                    }
                }
            };

            contracts = match p.contract {
                Some(Contract::Petite) => {
                    println!("Petite");
                    p.contract = Some(Contract::Petite);
                    vec![Contract::Pass, Contract::Garde, Contract::GardeSans, Contract::GardeContre]
                },
                Some(Contract::Garde) => {
                    println!("Garde");
                    p.contract = Some(Contract::Garde);
                    vec![Contract::Pass, Contract::GardeSans, Contract::GardeContre]
                },
                Some(Contract::GardeSans) => {
                    println!("Garde sans!");
                    p.contract = Some(Contract::GardeSans);
                    vec![Contract::Pass, Contract::GardeContre]
                },
                Some(Contract::GardeContre) =>
                {
                    println!("Garde contre : others must pass");
                    p.contract = Some(Contract::GardeContre);
                    vec![Contract::Pass]
                },
                Some(Contract::Pass) => {
                    println!("Pass");
                    p.contract = Some(Contract::Pass);
                    contracts
                },
                _ => {
                    debug!("A contract must be available for everyone!");
                    Err(TarotErrorKind::InvalidCase)?
                }
            };
            if p.contract != Some(Contract::Pass) && p.announce_slam() {
                slam_index = Some(i);
            }
        }
        // RULE: player who slammed must start
        if let Some(slammer) = slam_index {
            self.players.rotate_left(slammer);
        }
        Ok(())
    }
    pub fn passed(&self) -> bool {
        self.players.iter().all(|p| p.contract == Some(Contract::Pass))
    }
    pub fn finished(&self) -> bool {
        self.players.iter().all(|p| p.hand.is_empty())
    }
    pub fn discard (&mut self) -> Result<(), Error> {
        if self.passed() {
            Err(TarotErrorKind::NoTaker)?;
        }

        let mut callee: Option<Card> = None;
        let mut contract: Option<Contract> = None;
        if let Some(taker) = self.players.iter_mut().max_by(|c1, c2| c1.contract.unwrap().cmp(&c2.contract.unwrap())) {
            println!("{} has taken", &taker);
            contract = taker.contract;
            if let Mode::Five = self.mode {
                callee = taker.call()?;
            }
        }

        for p in &mut self.players {
            p.callee = callee;
            p.team = Some(Team::Defense);
            p.role = Some(Role::Defenser);
            if p.contract == contract {
                p.team = Some(Team::Attack);
                p.role = Some(Role::Taker);
            } else if let Some(ref card) = callee {
                if p.hand.0.contains(&&card) {
                    p.team = Some(Team::Attack);
                    p.role = Some(Role::Ally);
                }
            }
            p.contract = contract;
        }

        let team_partitioner = |p: &'_ &mut Player| -> bool {
            match &p.team {
                Some(team) => team == &Team::Attack,
                _ => false
            }
        };

        let (takers, others): (Vec<_>, Vec<_>) = self.players.iter_mut().partition(team_partitioner);
        for taker in takers {
            if taker.role != Some(Role::Taker) {
                continue
            }

            match taker.contract {
                Some(Contract::Pass) => continue,
                Some(Contract::GardeSans) => {
                    taker.owned.append(self.dog.give_all());
                    return Ok(())
                }
                Some(Contract::GardeContre) => {
                    for o in others {
                        o.owned.append(self.dog.give_all());
                    }
                    return Ok(())
                },
                _ => {
                    let discard = self.dog.len();
                    println!("In the dog, there was : {}", &self.dog);
                    taker.hand.append(self.dog.give_all());
                    taker.discard(discard);
                },
            }
        }
        Ok(())
    }
    pub fn play (&mut self) -> Result<(), Error> {
        let mut turn = Turn::default();
        let mut master_player: usize = 0;
        for (i, p) in self.players.iter_mut().enumerate() {
            if p.is_first_turn() {
                p.announce_handle();
            }
            println!("{}", &turn);
            println!("Hand of {} : {}", &p, &p.hand);
            println!("Choices :");
            let choices = &p.choices(&turn);
            if choices.is_empty() {
                println!("No choices available, invalid case.");
                Err(TarotErrorKind::InvalidCase)?
            }
            for &i in choices {
                println!("\t{0: <2} : {1}", &i, p.hand.0[i]);
            }

            if let Some(master) = turn.master_card() {
                println!("{} must play color {}", &p.name, &master)
            } else {
                println!("{} is first to play:", &p.name)
            }

            let index = if self.auto && choices.len() == 1 {
                choices[0]
            } else if self.random {
                choices[rand::thread_rng().gen_range(0, choices.len())]
            } else {
                loop {
                    match read_index() {
                        Ok(value) => if choices.contains(&value) {
                            break value
                        } else {
                            println!("Bad input, please retry")
                        },
                        _ => {
                            println!("Bad input, please retry")
                        }
                    }
                }
            };

            let card = p.give_one(index);
            if card.is_fool() {
                if p.last_turn() {
                    turn.put(card);
                    match p.team {
                        Some(Team::Attack)  => {
                            if self.attack_cards == MAX_CARDS - self.mode.dog_size() {
                                turn.master_index = Some(turn.len()-1);
                                master_player = i;
                            }
                        },
                        Some(Team::Defense) => {
                            if self.defense_cards == MAX_CARDS - self.mode.dog_size() {
                                turn.master_index = Some(turn.len()-1);
                                master_player = i;
                            }
                        },
                        _ => {
                            Err(TarotErrorKind::NoTeam)?
                        }
                    }
                } else {
                    p.owned.push(card);
                }
            } else {
                turn.put(card);
                if let Some(master) = turn.master_card() {
                    if master.master(card) {
                        println!("Master card is {}, so player {} stays master", master, master_player);
                    } else {
                        println!("Master card is {}, so player {} becomes master", card, i);
                        master_player = i;
                        turn.master_index = Some(turn.len()-1);
                    }
                } else {
                    println!("First card is {}, so player {} becomes master", card, i);
                    master_player = i;
                    turn.master_index = Some(turn.len()-1);
                }
            }
        }

        let cards = turn.take();
        println!("Winner is player {}", self.players[master_player]);
        // RULE: petit au bout works for last turn, or before last turn if a slam is occuring
        if cards.has_petit() &&
            (self.players[master_player].last_turn() ||
             (self.players[master_player].before_last_turn() &&
              ((self.attack_cards == MAX_CARDS - self.mode.dog_size() - self.mode as usize ) || (self.defense_cards == MAX_CARDS - self.mode.dog_size() - self.mode as usize)))) {
            println!("{} has Petit in last turn (Petit au bout) : +10 points", self.players[master_player]);
            self.petit_au_bout = self.players[master_player].team.clone();
        }
        match self.players[master_player].team {
            Some(Team::Attack) => self.attack_cards += cards.len(),
            Some(Team::Defense) => self.defense_cards += cards.len(),
            _ => Err(TarotErrorKind::NoTeam)?
        }
        self.players[master_player].owned.append(cards);
        self.players.rotate_left(master_player);
        Ok(())
    }
    pub fn count_points(&mut self) -> Result<(), Error> {
        if self.passed() {
            Err(TarotErrorKind::NoTaker)?;
        }
        let mut taker_index : Option<usize> = None;
        let mut ally_index : Option<usize> = None;
        let mut defense : Vec<usize> = Vec::new();
        let mut owning_card_player_index : Option<usize> = None;
        let mut missing_card_player_index : Option<usize> = None;
        let mut handle_bonus = 0.0;
        for (i, p) in self.players.iter().enumerate() {
            if p.owe_card() {
                owning_card_player_index = Some(i);
            }
            if p.missing_card() {
                missing_card_player_index = Some(i);
            }
            if let Some(handle) = &p.handle {
                handle_bonus += f64::from(handle.clone() as u8);
                println!("Handle bonus: {}", handle_bonus);
            }
            match p.role {
                Some(Role::Taker) => {
                    taker_index = Some(i)
                }
                Some(Role::Ally) => {
                    ally_index = Some(i)
                }
                Some(Role::Defenser) => {
                    defense.push(i)
                }
                _ => Err(TarotErrorKind::InvalidCase)?,
            }
        }
        if let Some(owning_index) = owning_card_player_index {
            let low_card = self.players[owning_index].give_low();
            if let Some(low) = low_card {
                if let Some(missing_index) = missing_card_player_index {
                    self.players[missing_index].owned.push(low);
                }
            }
        }
        if let Some(ally_index) = ally_index {
            let ally_cards = self.players[ally_index].owned.give_all();
            if let Some(taker_index) = taker_index {
                self.players[taker_index].owned.append(ally_cards)
            } else {
                println!("Cant merge cards of ally if no taker");
                Err(TarotErrorKind::NoTaker)?;
            }
        }

        if let Some(taker_index) = taker_index {
            let slam_bonus = self.players[taker_index].slam_bonus();
            println!("Taker slam bonus: {}", &slam_bonus);
            let contract_points = self.players[taker_index].contract_points()?;
            println!("Taker contract points: {}", &contract_points);
            let mut petit_au_bout_bonus = 0.0;
            if let Some(contract) = self.players[taker_index].contract {
                petit_au_bout_bonus = match self.petit_au_bout {
                    Some(Team::Defense) => {
                        println!("Petit au bout for defense: {}", -10.0 * f64::from(contract as u8));
                        -10.0 * f64::from(contract as u8)
                    },
                    Some(Team::Attack) => {
                        println!("Petit au bout for attack: {}", 10.0 * f64::from(contract as u8));
                        10.0 * f64::from(contract as u8)
                    },
                    _ => 0.0
                };
            } else {
                Err(TarotErrorKind::NoContract)?
            }

            let ratio = match self.mode {
                Mode::Three => 2.0,
                Mode::Four => 3.0,
                Mode::Five => if ally_index.is_none() { 4.0 } else { 2.0 },
            };

            if contract_points < 0.0 {
                handle_bonus *= -1.0;
            }
            println!("Attack handle bonus: {}", &handle_bonus);

            let points = contract_points + petit_au_bout_bonus + handle_bonus + slam_bonus;
            println!("Taker points: {}", &points);

            self.players[taker_index].total = ratio * points;
            println!("Taker total points: {}", &self.players[taker_index].total);

            if let Some(ally_index) = ally_index {
                self.players[ally_index].total = points;
                println!("Ally total points: {}", &self.players[ally_index].total);
            }

            for defenser_index in defense {
                self.players[defenser_index].total = -1.0 * points;
                println!("Defense total points: {}", &self.players[defenser_index].total);
            }
            //if handle_bonus != 0.0  && petit_au_bout_bonus != 0.0 && slam_bonus != 0.0 && ratio == 4.0 {
            //    helpers::wait_input();
            //}
        } else {
            println!("Cant count points if no taker");
            Err(TarotErrorKind::NoTaker)?;
        }
        self.is_consistent();
        Ok(())
    }
}

#[test]
fn game_tests() {
    use crate::mode::*;
    test_game(Mode::Three);
    test_game(Mode::Four);
    test_game(Mode::Five);
}