
#[derive(Debug, PartialEq, Clone)]
pub struct Game {
    pub id: u32,
    pub name: String,
    pub rating: f64
}

impl Game {
    pub fn new(id: u32, name: String) -> Game {
        Game { id, name, rating: 0.0 }
    }
}
