mod song;
mod uge;

fn main() {
    let data = std::fs::read("/tmp/gbow_startham_forest.uge").expect("Failed to read UGE file");
    let song = match uge::parse_song(&data) {
        Ok(song) => song,
        Err(err) => {
            eprintln!("{err}");
            return;
        }
    };
}
