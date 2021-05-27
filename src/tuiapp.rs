use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use itertools::{izip, Itertools};
use octocrab::models::repos::Release;
use std::convert::TryInto;
use std::io::stdout;
use std::ops::{Range, Rem};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};
use tui::widgets::{
    self, Axis, Block, BorderType, Borders, Chart, Dataset, GraphType, LineGauge, Paragraph, Row,
    Wrap,
};
use tui::{backend::CrosstermBackend, Terminal};
use tui::{
    layout::Direction,
    style::{Color, Modifier, Style},
};
use tui::{
    layout::{Alignment, Constraint, Layout, Rect},
    text::Text,
};
use tui::{symbols, Frame};
use tui::{
    text::{Span, Spans},
    widgets::Table,
};

use crate::Args;

type Backend = CrosstermBackend<std::io::Stdout>;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Event {
    Tick,
    Input(KeyEvent),
}

#[derive(Debug)]
struct TuiApp {
    args: Args,

    desc_box_size: u16,

    field_selected: usize,
    owner: String,
    repo: String,
    release: String,
    asset: String,

    selected_col: usize,
    selected_asset: usize,
    selected_rel: usize,
    found_releases: Vec<Release>,
    found_assets: Vec<Release>,
}

struct Areas {
    top_area: Rect,

    owner_key: Rect,
    repo_key: Rect,
    release_key: Rect,
    asset_key: Rect,

    owner_field: Rect,
    repo_field: Rect,
    release_field: Rect,
    asset_field: Rect,

    found_assets: Rect,
    found_releases: Rect,
    description: Rect,

    buttons: Vec<Rect>,
}

impl Areas {
    fn new(total: Rect, app: &TuiApp) -> Self {
        let block = TuiApp::block();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2 + 2),
                Constraint::Min(0),
                Constraint::Length(1),
                Constraint::Length(2 + app.desc_box_size),
            ])
            .split(total);
        let (topbar, body, buttons, bottom) = (chunks[0], chunks[1], chunks[2], chunks[3]);

        let body = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
            .split(body);

        let buttons = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
            .split(buttons);

        let topbarsplit = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
            .split(block.inner(topbar));
        let (toprow1, toprow2) = (topbarsplit[0], topbarsplit[1]);

        let split = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Ratio(1, 4),
                Constraint::Ratio(1, 4),
                Constraint::Ratio(1, 4),
                Constraint::Ratio(1, 4),
            ]);
        let toprow = [split.split(toprow1), split.split(toprow2)];

        Self {
            top_area: topbar,
            owner_key: toprow[0][0],
            owner_field: toprow[0][1],
            repo_key: toprow[0][2],
            repo_field: toprow[0][3],
            release_key: toprow[1][0],
            release_field: toprow[1][1],
            asset_key: toprow[1][2],
            asset_field: toprow[1][3],
            found_assets: body[0],
            found_releases: body[1],
            buttons,
            description: bottom,
        }
    }
}

impl TuiApp {
    fn new(args: Args) -> Self {
        Self {
            args,
            owner: String::from("<search>"),
            repo: String::from("<search>"),
            release: String::from("<search>"),
            asset: String::from("<search>"),
            desc_box_size: 10,
            field_selected: 0,
            found_releases: Vec::new(),
            found_assets: Vec::new(),
            selected_col: 0,
            selected_rel: 0,
            selected_asset: 0,
        }
    }

    fn draw(&self, f: &mut Frame<Backend>) {
        let chunks = Areas::new(f.size(), self);

        let block = Self::block();
        f.render_widget(block.clone(), chunks.top_area);
        f.render_widget(block.clone().title("Releases"), chunks.found_releases);
        f.render_widget(block.clone().title("Assets"), chunks.found_assets);
        f.render_widget(block.title("Description"), chunks.description);

        let text = |t, s| Paragraph::new(Text::styled(t, s));

        let key_style = Style::default()
            .fg(Color::White)
            .bg(Color::Black)
            .add_modifier(Modifier::BOLD);
        f.render_widget(text("Owner", key_style), chunks.owner_key);
        f.render_widget(text("Repo", key_style), chunks.repo_key);
        f.render_widget(text("Release", key_style), chunks.release_key);
        f.render_widget(text("Asset", key_style), chunks.asset_key);

        let field_style = Style::default()
            .fg(Color::White)
            .bg(Color::Black)
            .add_modifier(Modifier::UNDERLINED);
        f.render_widget(text(&self.owner, field_style), chunks.owner_field);
        f.render_widget(text(&self.repo, field_style), chunks.repo_field);
        f.render_widget(text(&self.release, field_style), chunks.release_field);
        f.render_widget(text(&self.asset, field_style), chunks.asset_field);

        let button_style = Style::default()
            .fg(Color::White)
            .bg(Color::Blue)
            .add_modifier(Modifier::BOLD);
        f.render_widget(text("Install", button_style), chunks.buttons[0]);
        f.render_widget(text("Link", button_style), chunks.buttons[1]);

        let releases = Table::new(
            self.found_releases
                .iter()
                .map(|rel| {
                    Row::new(vec![
                        rel.name.clone().unwrap_or(String::new()),
                        rel.tag_name.clone(),
                        rel.published_at.to_string(),
                    ])
                })
                .collect_vec(),
        )
        .header(Row::new(vec!["name", "tag_name", "published_at"]).bottom_margin(1));
        f.render_widget(releases, chunks.found_releases);

        let assets = Table::new(
            self.found_releases
                .get(self.selected_rel)
                .map(|x| {
                    x.assets
                        .iter()
                        .map(|ass| {
                            Row::new(vec![
                                ass.label.clone().unwrap_or(String::new()),
                                ass.name.clone(),
                                ass.id.to_string(),
                            ])
                        })
                        .collect_vec()
                })
                .unwrap_or(vec![]),
        )
        .header(Row::new(vec!["label", "name", "id"]).bottom_margin(1));
        f.render_widget(assets, chunks.found_assets);

        // TODO: format the whole description
        let desc = match self.selected_col {
            0 => {
                //Releases
                let body = self
                    .found_releases
                    .get(self.selected_rel)
                    .and_then(|x| x.body.as_ref())
                    .map(|x| x.as_str())
                    .unwrap_or("");
                Text::raw(body)
            }
            1 => {
                //Assets
                let body = self.found_releases[self.selected_rel].assets[self.selected_asset]
                    .browser_download_url
                    .to_string();
                Text::raw(body)
            }
            _ => Text::raw(""),
        };
    }

    fn on_key(&mut self, key: KeyCode) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    fn on_tick(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    fn block() -> tui::widgets::Block<'static> {
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White))
            .border_type(BorderType::Rounded)
            .style(Style::default().bg(Color::Black))
    }
}

pub fn tui(args: Args) -> Result<(), Box<dyn std::error::Error>> {
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
