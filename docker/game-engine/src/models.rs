use diesel::prelude::*;

#[derive(Insertable)]
#[diesel(table_name = crate::schema::game_rounds)]
pub struct NewGameRound<'a> {
    pub table_id: i32,
    pub round_number: i32,
    pub dealer_points: i32,
    pub result_json: &'a str,
}
