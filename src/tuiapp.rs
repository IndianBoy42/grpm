use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use itertools::{izip, Itertools};
use std::convert::TryInto;
use std::io::stdout;
use std::ops::{Range, Rem};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};
use tui::layout::{Alignment, Constraint, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans};
use tui::widgets::{
    self, Axis, Block, BorderType, Borders, Chart, Dataset, GraphType, LineGauge, Paragraph, Wrap,
};
use tui::{backend::CrosstermBackend, Terminal};
use tui::{symbols, Frame};

use crate::Args;

type Backend = CrosstermBackend<std::io::Stdout>;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Event {
    Tick,
    Input(KeyEvent),
}


#[derive(Debug)]
struct TuiApp {}

impl TuiApp {
    fn new(args: Args) -> Self {
        Self {}
    }

    fn draw(&self, f: &mut Frame<Backend>) {}

    fn on_key(&mut self, key: KeyCode) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    fn on_tick(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

fn tui(args: Args) -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;

    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);

    let mut terminal = Terminal::new(backend)?;

    let rx = input_handling_thread(&terminal);

    let mut app = TuiApp::new(args);

    terminal.clear()?;

    loop {
        terminal.draw(|f| app.draw(f))?;

        match rx.recv()? {
            Event::Tick => app.on_tick()?,
            Event::Input(key) => match key.code {
                KeyCode::Char('q') => {
                    disable_raw_mode()?;
                    execute!(
                        terminal.backend_mut(),
                        LeaveAlternateScreen,
                        DisableMouseCapture
                    )?;
                    terminal.show_cursor()?;
                    break;
                }
                code => app.on_key(code)?,
            },
        };
    }

    Ok(())
}

fn input_handling_thread(_terminal: &Terminal<Backend>) -> Receiver<Event> {
    let (tx, rx) = mpsc::channel();

    let tick_rate = Duration::from_millis(1000 / 60);
    thread::spawn(move || {
        let mut last_tick = Instant::now();
        loop {
            // Poll for tick rate duration, if no events, sent tick event.
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            // Poll for events
            if event::poll(timeout).unwrap() {
                if let CEvent::Key(key) = event::read().unwrap() {
                    tx.send(Event::Input(key)).unwrap();
                }
            }

            // Send tick event regularly
            if last_tick.elapsed() >= tick_rate {
                tx.send(Event::Tick).unwrap();
                last_tick = Instant::now();
            }
        }
    });

    rx
}
