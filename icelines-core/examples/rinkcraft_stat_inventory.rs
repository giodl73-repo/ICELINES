//! Emit the canonical metric inventory; documentary specs can lag the code.
use icelines_core::{model::Position, stats_catalog::StatId};
fn main() {
    let rows:Vec<_>=StatId::all().iter().map(|s|serde_json::json!({"id":format!("{s:?}"),"key":s.cli_key(),"label":s.label(),"category":format!("{:?}",s.category()),"unit":format!("{:?}",s.unit()),"higher_is_better":s.higher_is_better(),"source":s.report_source().map(|r|r.url_path()),"available_since":s.available_since().0,"center":s.applies_to(Position::Center,false),"defense":s.applies_to(Position::Defense,false),"goalie":s.applies_to(Position::Goalie,true)})).collect();
    println!("{}", serde_json::to_string_pretty(&rows).unwrap());
}
