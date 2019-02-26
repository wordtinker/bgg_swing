
#[derive(Debug, PartialEq, Clone)]
pub struct Game {
    pub id: u32,
    pub name: String,
    pub rating: f64,
    pub votes: u32,
    pub bgg_num_votes: u32,
    pub bgg_geek_rating: f64,
    pub bgg_avg_rating: f64
}

impl Game {
    pub fn new(id: u32, name: String) -> Game {
        Game { id, name, rating: 0.0, votes: 0, bgg_num_votes: 0, bgg_geek_rating: 0.0, bgg_avg_rating: 0.0 }
    }
}

pub type User = String; // user name
