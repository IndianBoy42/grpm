use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::executor::block_on;
use itertools::Itertools;
use octocrab::models::repos::{Asset, Release};
use regex::Regex;
use std::{
    io::stdout,
    sync::mpsc::{Receiver, Sender},
};
use std::{ops::Add, sync::mpsc};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use std::{sync::Mutex, thread};
use tui::widgets::Table;
use tui::widgets::{Block, BorderType, Borders, Paragraph, Row, Wrap};
use tui::Frame;
use tui::{backend::CrosstermBackend, Terminal};
use tui::{
    layout::Direction,
    style::{Color, Modifier, Style},
};
use tui::{
    layout::{Alignment, Constraint, Layout, Rect},
    text::Text,
};

use crate::{common, Args};

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
    search_rels: String,
    search_assets: String,

    release_re: Option<Regex>,
    asset_re: Option<Regex>,

    selected_col: usize,
    selected_asset: usize,
    selected_release: usize,
    all_releases: Vec<Release>,
    found_releases: Vec<Release>,
    found_assets: Vec<Asset>,
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
    fn selected_field(&self) -> &str {
        match self.field_selected {
            0 => &self.owner,
            1 => &self.repo,
            2 => &self.search_rels,
            3 => &self.search_assets,
            _ => panic!("Invalid field"),
        }
    }
    fn selected_field_mut(&mut self) -> &mut String {
        match self.field_selected {
            0 => &mut self.owner,
            1 => &mut self.repo,
            2 => &mut self.search_rels,
            3 => &mut self.search_assets,
            _ => panic!("Invalid field"),
        }
    }
    fn new(args: Args) -> Self {
        Self {
            owner: args.owner.clone().unwrap_or(String::from("<search>")),
            repo: args.repo.clone().unwrap_or(String::from("<search>")),
            search_rels: args.release.clone().unwrap_or(String::from("<search>")),
            search_assets: args.asset.clone().unwrap_or(String::from("<search>")),
            desc_box_size: 10,
            field_selected: 0,
            found_releases: Vec::new(),
            all_releases: Vec::new(),
            found_assets: Vec::new(),
            selected_col: 0,
            selected_release: 0,
            selected_asset: 0,
            release_re: None,
            asset_re: None,
            args,
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

        let field_style = |i| {
            let a = Style::default().fg(Color::White).bg(Color::Black);
            if self.field_selected == i {
                a.add_modifier(Modifier::UNDERLINED)
            } else {
                a
            }
        };
        f.render_widget(text(&self.owner, field_style(0)), chunks.owner_field);
        f.render_widget(text(&self.repo, field_style(1)), chunks.repo_field);
        f.render_widget(
            text(&self.search_rels, field_style(2)),
            chunks.release_field,
        );
        f.render_widget(
            text(&self.search_assets, field_style(3)),
            chunks.asset_field,
        );

        let button_style = Style::default()
            .fg(Color::White)
            .bg(Color::Blue)
            .add_modifier(Modifier::BOLD);
        f.render_widget(
            text("Install", button_style).alignment(Alignment::Center),
            chunks.buttons[0],
        );
        f.render_widget(
            text("Link", button_style).alignment(Alignment::Center),
            chunks.buttons[1],
        );

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
                .get(self.selected_release)
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
                    .get(self.selected_release)
                    .and_then(|x| x.body.as_ref())
                    .map(|x| x.as_str())
                    .unwrap_or("");
                Text::raw(body)
            }
            1 => {
                //Assets
                let body = self.found_releases[self.selected_release].assets[self.selected_asset]
                    .browser_download_url
                    .to_string();
                Text::raw(body)
            }
            _ => Text::raw(""),
        };
        let desc = Paragraph::new(desc).wrap(Wrap { trim: false });
        f.render_widget(desc, chunks.description);
    }

    fn update_release_re(&mut self, recompile: bool) -> Result<(), Box<dyn std::error::Error>> {
        if recompile {
            self.release_re = Some(Regex::new(&self.search_rels)?);
        }
        self.found_releases = if let Some(re) = &self.release_re {
            common::find_release_from(&re, &self.all_releases)
        } else {
            self.all_releases.clone()
        };
        self.selected_release = 0;
        self.selected_asset = 0;
        self.update_asset_re(false)?;
        Ok(())
    }
    fn update_asset_re(&mut self, recompile: bool) -> Result<(), Box<dyn std::error::Error>> {
        if recompile {
            self.asset_re = Some(Regex::new(&self.search_assets)?);
        }
        self.found_assets = if let Some(re) = &self.asset_re {
            common::find_asset_from(&re, &self.found_releases[self.selected_release].assets)
        } else {
            self.found_releases[self.selected_release].assets.clone()
        };
        Ok(())
    }

    fn on_key(&mut self, key: KeyCode) -> Result<(), Box<dyn std::error::Error>> {
        match key {
            KeyCode::Char(c) => {
                let f = self.selected_field_mut();
                if f == "<search>" {
                    f.clear();
                }
                f.push(c);

                match self.field_selected {
                    // Only Update the repo on Enter
                    0 | 1 => {}
                    // Update the regexes
                    2 => self.update_release_re(true)?,
                    3 => self.update_asset_re(true)?,
                    _ => panic!("Invalid field"),
                };
            }
            KeyCode::Backspace => {
                let f = self.selected_field_mut();
                if f == "<search>" {
                    f.clear();
                }
                f.pop();

                match self.field_selected {
                    // Only Update the repo on Enter
                    0 | 1 => {}
                    // Update the regexes
                    2 => self.update_release_re(true)?,
                    3 => self.update_asset_re(true)?,
                    _ => panic!("Invalid field"),
                };
            }
            KeyCode::Enter => {
                match self.field_selected {
                    // Update the repo
                    0 | 1 => {
                        self.selected_col = 0;
                        self.selected_asset = 0;
                        self.selected_release = 0;
                        self.all_releases =
                            // TODO: move this to a different thread?
                            block_on(common::list_releases(&self.owner, &self.repo))?;
                        self.found_releases = self.all_releases.clone();
                    }
                    2 => self.update_release_re(true)?,
                    3 => self.update_asset_re(true)?,
                    _ => panic!("Invalid field"),
                };
            }
            KeyCode::Tab | KeyCode::Left => {
                self.field_selected = self.field_selected.saturating_sub(1);
            }
            KeyCode::Right => {
                self.field_selected = self.field_selected.add(1).min(3);
            }
            KeyCode::Up => {
                self.field_selected = match self.field_selected {
                    i @ (2 | 3) => i - 2,
                    i => i,
                };
            }
            KeyCode::Down => {
                self.field_selected = match self.field_selected {
                    i @ (0 | 1) => i + 2,
                    i => i,
                };
            }
            KeyCode::Home => {}
            KeyCode::End => {}
            KeyCode::PageUp => {}
            KeyCode::PageDown => {}
            KeyCode::BackTab => {}
            KeyCode::Delete => {}
            KeyCode::Insert => {}
            KeyCode::F(_) => {}
            KeyCode::Null => {}
            KeyCode::Esc => {}
        };
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
    let (tx, rx) = input_handling_thread(&terminal);
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

fn input_handling_thread(
    _terminal: &Terminal<Backend>,
) -> (Arc<Mutex<Sender<Event>>>, Receiver<Event>) {
    let (tx, rx) = mpsc::channel();
    let tx = Arc::new(Mutex::new(tx));

    let tick_rate = Duration::from_millis(1000 / 60);
    {
        let tx = tx.clone();
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
                        tx.lock().unwrap().send(Event::Input(key)).unwrap();
                    }
                }

                // Send tick event regularly
                if last_tick.elapsed() >= tick_rate {
                    tx.lock().unwrap().send(Event::Tick).unwrap();
                    last_tick = Instant::now();
                }
            }
        });
    }

    (tx, rx)
}

// fn downloading_thread(_terminal: &Terminal<Backend>, tx: Arc<Mutex<Sender<Event>>>, rx: Receiver<Download>) {
//     let tick_rate = Duration::from_millis(1000 / 60);
//     thread::spawn(move || {
//         let mut last_tick = Instant::now();
//         loop {}
//     });
// }
