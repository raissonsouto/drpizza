use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::models::BusinessHours;
use colored::*;

pub struct Spinner {
    running: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl Spinner {
    pub fn new(msg: &str) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let r = running.clone();
        let msg = msg.to_string();
        let handle = thread::spawn(move || {
            let frames = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
            let mut i = 0;
            while r.load(Ordering::Relaxed) {
                print!("\r{} {} ", frames[i].to_string().cyan(), msg);
                io::stdout().flush().ok();
                i = (i + 1) % frames.len();
                thread::sleep(Duration::from_millis(80));
            }
            print!("\r{}\r", " ".repeat(msg.len() + 4));
            io::stdout().flush().ok();
        });
        Spinner {
            running,
            handle: Some(handle),
        }
    }

    pub fn stop(mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            h.join().ok();
        }
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            h.join().ok();
        }
    }
}

pub fn read_input(prompt: &str) -> String {
    print!("{}", prompt);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

pub fn read_int(prompt: &str) -> Option<u32> {
    let input = read_input(prompt);
    input.parse::<u32>().ok()
}

pub fn get_category_icon(name: &str) -> &str {
    let n = name.to_lowercase();
    if n.contains("pizza") {
        "🍕"
    } else if n.contains("bebida") || n.contains("refrigerante") || n.contains("suco") {
        "🥤"
    } else if n.contains("combo") {
        "📦"
    } else if n.contains("bread") || n.contains("pão") || n.contains("pao") {
        "🥖"
    } else {
        "🍽️"
    }
}

pub fn translate_status(status: &str) -> &str {
    match status {
        "created" => "Criado",
        "pending_online_payment" => "Aguardando pagamento",
        "waiting_confirmation" => "Aguardando confirmação",
        "confirmed" => "Confirmado",
        "released" => "Saiu para entrega",
        "concluded" => "Concluído",
        "cancelled" => "Cancelado",
        _ => status,
    }
}

pub fn today_weekday() -> String {
    chrono::Local::now()
        .format("%A")
        .to_string()
        .to_lowercase()
}

pub fn get_day_hours<'a>(
    bh: &'a BusinessHours,
    day: &str,
) -> Option<&'a Vec<Vec<String>>> {
    match day {
        d if d.starts_with("sun") || d.starts_with("dom") => bh.sunday.as_ref(),
        d if d.starts_with("mon") || d.starts_with("seg") => bh.monday.as_ref(),
        d if d.starts_with("tue") || d.starts_with("ter") => bh.tuesday.as_ref(),
        d if d.starts_with("wed") || d.starts_with("qua") => bh.wednesday.as_ref(),
        d if d.starts_with("thu") || d.starts_with("qui") => bh.thursday.as_ref(),
        d if d.starts_with("fri") || d.starts_with("sex") => bh.friday.as_ref(),
        d if d.starts_with("sat") || d.starts_with("sab") || d.starts_with("sáb") => {
            bh.saturday.as_ref()
        }
        _ => None,
    }
}

pub fn format_phone(raw: &str) -> String {
    let digits: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
    let digits = if digits.len() > 11 && digits.starts_with("55") {
        &digits[2..]
    } else {
        &digits
    };
    match digits.len() {
        11 => format!(
            "({}) {}-{}",
            &digits[0..2],
            &digits[2..7],
            &digits[7..11]
        ),
        10 => format!(
            "({}) {}-{}",
            &digits[0..2],
            &digits[2..6],
            &digits[6..10]
        ),
        _ => raw.to_string(),
    }
}

pub fn print_day(label: &str, hours: &Option<Vec<Vec<String>>>) {
    match hours {
        Some(slots) if !slots.is_empty() => {
            let formatted: Vec<String> = slots
                .iter()
                .map(|slot| {
                    if slot.len() == 2 {
                        format!("{} - {}", slot[0], slot[1])
                    } else {
                        slot.join(", ")
                    }
                })
                .collect();
            println!("{}: {}", label, formatted.join(" | ").green());
        }
        _ => {
            println!("{}: {}", label, "Fechado".red());
        }
    }
}
