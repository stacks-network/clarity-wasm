#[derive(Debug)]
pub struct Block {
    pub height: u32,
    pub hash: String,
    pub timestamp: String,
    pub tx_count: u32
}

#[derive(Debug)]
pub struct Environment<'a> {
    pub id: i32,
    pub name: &'a str,
}