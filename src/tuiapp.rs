use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use itertools::Itertools;
use octocrab::models::repos::{Asset, Release};
use regex::Regex;
use std::{
    convert::TryInto,
    io::stdout,
    mem::MaybeUninit,
    sync::mpsc::{Receiver, Sender},
};
use std::{ops::Add, sync::mpsc};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use std::{sync::Mutex, thread};
use tokio::runtime::Runtime;
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

use crate::{common, ArgFlags, Args};

type Backend = CrosstermBackend<std::io::Stdout>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Event {
    Tick,
    Input(KeyEvent),
}
#[derive(Debug, Clone, PartialEq)]
enum DownloadPlease {
    Releases(String, String),
    Asset(Asset),
}

#[derive(Debug)]
struct TuiApp {
    args: ArgFlags,

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

    try_dl_repo: Sender<DownloadPlease>,
    get_dl_repo: Receiver<Vec<Release>>,
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

fn evensplit<const N: usize>() -> [Constraint; N] {
    let mut arr = MaybeUninit::uninit_array();
    for i in 0..N {
        arr[i] = MaybeUninit::new(Constraint::Ratio(1, N.try_into().unwrap()));
    }
    unsafe { MaybeUninit::array_assume_init(arr) }
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
            .constraints(evensplit::<4>())
            .split(buttons);

        let topbarsplit = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
            .split(block.inner(topbar));
        let (toprow1, toprow2) = (topbarsplit[0], topbarsplit[1]);

        let split = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(evensplit::<4>());
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
            found_releases: body[0],
            found_assets: body[1],
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
    fn new(
        args: Args,
        try_dl_repo: Sender<DownloadPlease>,
        get_dl_repo: Receiver<Vec<Release>>,
    ) -> Self {
        let app = Self {
            owner: args.owner.clone().unwrap_or(String::from(".*")),
            repo: args.repo.clone().unwrap_or(String::from("*")),
            search_rels: args.release.clone().unwrap_or(String::from("*")),
            search_assets: args.asset.clone().unwrap_or(String::from("*")),
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
            args: args.flags,
            try_dl_repo,
            get_dl_repo,
        };
        if let (Some(_), Some(_)) = (args.owner, args.repo) {
            app.update_release_list().unwrap();
            // if let Some(_) = args.release {
            //     app.update_release_re(true).unwrap();
            // }
            // if let Some(_) = args.asset {
            //     app.update_asset_re(true).unwrap();
            // }
        }
        app
    }

    fn draw(&self, f: &mut Frame<Backend>) {
        let chunks = Areas::new(f.size(), self);

        let block = Self::block();
        f.render_widget(block.clone(), chunks.top_area);

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
            text("Download", button_style).alignment(Alignment::Center),
            chunks.buttons[1],
        );
        f.render_widget(
            text("Save", button_style).alignment(Alignment::Center),
            chunks.buttons[2],
        );
        f.render_widget(
            text("Link", button_style).alignment(Alignment::Center),
            chunks.buttons[3],
        );

        // TODO: do pagination (and integrate with the downloader to reduce the release fetch time)
        let releases = Table::new(
            self.found_releases
                .iter()
                // .take(10)
                .map(|rel| {
                    Row::new(vec![
                        rel.tag_name.clone(),
                        rel.published_at.to_string(),
                        rel.name.clone().unwrap_or(String::from("N/A")),
                    ])
                })
                .collect_vec(),
        )
        .widths(&[
            Constraint::Percentage(50), // TODO: How to chose the lengths? why Min(0) doesnt work...
            Constraint::Length(10),
            Constraint::Max(10),
        ])
        .header(Row::new(vec!["tag_name", "published_at", "name"]));
        f.render_widget(
            releases.block(block.clone().title("Releases")),
            chunks.found_releases,
        );

        let assets = Table::new(
            self.found_assets
                .iter()
                // .take(10)
                .map(|ass| {
                    Row::new(vec![
                        ass.name.clone(),
                        ass.label.clone().unwrap_or(String::from("N/A")),
                        ass.id.to_string(),
                    ])
                })
                .collect_vec(),
        )
        .widths(&[
            Constraint::Percentage(50), // TODO: How to chose the lengths? why Min(0) doesnt work...
            Constraint::Length(10),
            Constraint::Min(10),
        ])
        .header(Row::new(vec!["name", "label", "id"]));
        f.render_widget(
            assets.block(block.clone().title("Assets")),
            chunks.found_assets,
        );

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
        f.render_widget(desc.block(block.title("Description")), chunks.description);
    }

    fn update_release_list(&self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(self.try_dl_repo.send(DownloadPlease::Releases(
            self.owner.clone(),
            self.repo.clone(),
        ))?)
    }
    fn update_release_re(&mut self, recompile: bool) -> Result<(), Box<dyn std::error::Error>> {
        if recompile {
            if self.search_rels == "" {
                self.release_re = Some(Regex::new(".*")?);
            } else {
                self.release_re = Some(Regex::new(&self.search_rels)?);
            }
        }
        // eprintln!("{:?}", self.all_releases);
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
            if self.search_assets == "" {
                self.asset_re = Some(Regex::new(".*")?);
            } else {
                self.asset_re = Some(Regex::new(&self.search_assets)?);
            }
        }
        self.found_assets = if let Some(re) = &self.asset_re {
            if let Some(selected_release) = self.found_releases.get(self.selected_release) {
                common::find_asset_from(&re, &selected_release.assets)
            } else {
                vec![]
            }
        } else {
            if let Some(selected_release) = self.found_releases.get(self.selected_release) {
                selected_release.assets.clone()
            } else {
                vec![]
            }
        };
        Ok(())
    }

    fn on_key(&mut self, key: KeyCode) -> Result<(), Box<dyn std::error::Error>> {
        match key {
            KeyCode::Char(c) => {
                let f = self.selected_field_mut();
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
                if f == ".*" {
                    f.clear();
                } else {
                    f.pop();
                    if f == "" {
                        *f = String::from(".*");
                    }
                }

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
                    0 | 1 => self.update_release_list()?,
                    2 => self.update_release_re(true)?,
                    3 => self.update_asset_re(true)?,
                    _ => panic!("Invalid field"),
                };
            }
            KeyCode::Left => {
                self.field_selected = self.field_selected.saturating_sub(1);
            }
            KeyCode::Tab | KeyCode::Right => {
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
        let rels = self.get_dl_repo.try_iter().last();
        if let Some(rels) = rels {
            self.all_releases = rels;
            self.found_releases = self.all_releases.clone();
            self.selected_col = 0;
            self.selected_asset = 0;
            self.selected_release = 0;
            self.update_release_re(true)?;
            self.update_asset_re(true)?;
        }
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
    let (send_repos, recv_rels) = downloading_thread(&terminal);
    let mut app = TuiApp::new(args, send_repos, recv_rels);

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

// TODO: Cache the Release list in `~/.cache/grpm` and only download the new releases
fn downloading_thread(
    _terminal: &Terminal<Backend>,
) -> (Sender<DownloadPlease>, Receiver<Vec<Release>>) {
    let (send_repos, recv_repos) = mpsc::channel();
    let (send_rels, recv_rels) = mpsc::channel();

    thread::spawn(move || {
        let rt = Runtime::new().unwrap();
        use DownloadPlease::*;
        while let Ok(req) = recv_repos.recv() {
            match req {
                Releases(owner, repo) => {
                    let rels = rt.block_on(common::list_releases(&owner, &repo)).unwrap();
                    send_rels.send(rels).unwrap();
                }
                Asset(ass) => {
                    // TODO: Download the asset
                }
            }
        }
    });

    (send_repos, recv_rels)
}
