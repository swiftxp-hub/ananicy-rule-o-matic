use crate::application::rule_service::RuleService;
use crate::domain::models::EnrichedRule;

use anyhow::Result;

use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};

use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};

use rust_i18n::t;
use std::{borrow::Cow, io};

struct App
{
    all_rules: Vec<EnrichedRule>,
    filtered_rules: Vec<EnrichedRule>,
    list_state: ListState,
    search_query: String,
    input_mode: InputMode,
    current_page: usize,
    items_per_page: usize,
}

#[derive(PartialEq)]
enum InputMode
{
    Normal,
    Editing,
}

impl App
{
    fn new(rules: Vec<EnrichedRule>) -> Self
    {
        let mut app = Self {
            all_rules: rules.clone(),
            filtered_rules: rules,
            list_state: ListState::default(),
            search_query: String::new(),
            input_mode: InputMode::Normal,
            current_page: 0,
            items_per_page: 50,
        };

        if !app.filtered_rules.is_empty()
        {
            app.list_state.select(Some(0));
        }

        app
    }

    fn update_search(&mut self)
    {
        let q = self.search_query.to_lowercase();

        self.filtered_rules = self
            .all_rules
            .iter()
            .filter(|r| {
                let name_match = r.data.name.as_deref().unwrap_or("").to_lowercase().contains(&q);
                let comment_match = r.context_comment.as_deref().unwrap_or("").to_lowercase().contains(&q);
                name_match || comment_match
            })
            .cloned()
            .collect();

        self.current_page = 0;

        if self.filtered_rules.is_empty()
        {
            self.list_state.select(None);
        }
        else
        {
            self.list_state.select(Some(0));
        }
    }

    fn next(&mut self)
    {
        if let Some(selected) = self.list_state.selected()
        {
            let start = self.current_page * self.items_per_page;
            let end = (start + self.items_per_page).min(self.filtered_rules.len());
            let count_on_page = end - start;

            if count_on_page > 0 && selected < count_on_page - 1
            {
                self.list_state.select(Some(selected + 1));
            }
        }
    }

    fn previous(&mut self)
    {
        if let Some(selected) = self.list_state.selected()
        {
            if selected > 0
            {
                self.list_state.select(Some(selected - 1));
            }
        }
    }

    fn next_page(&mut self)
    {
        let total_pages = (self.filtered_rules.len() as f64 / self.items_per_page as f64).ceil() as usize;
        if total_pages > 0 && self.current_page < total_pages - 1
        {
            self.current_page += 1;
            self.list_state.select(Some(0));
        }
    }

    fn previous_page(&mut self)
    {
        if self.current_page > 0
        {
            self.current_page -= 1;
            self.list_state.select(Some(0));
        }
    }
}

pub fn run_app(service: &RuleService) -> Result<()>
{
    let rules = service.search_rules("")?;

    enable_raw_mode()?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(rules);

    loop
    {
        terminal.draw(|frame| ui(frame, &mut app))?;

        if event::poll(std::time::Duration::from_millis(100))?
        {
            if let Event::Key(key) = event::read()?
            {
                match app.input_mode
                {
                    InputMode::Normal => match key.code
                    {
                        KeyCode::Char('q') | KeyCode::Esc => break,

                        KeyCode::Char('s') | KeyCode::Char('/') =>
                        {
                            app.input_mode = InputMode::Editing;
                        }

                        KeyCode::Down => app.next(),

                        KeyCode::Up => app.previous(),

                        KeyCode::Right => app.next_page(),

                        KeyCode::Left => app.previous_page(),

                        _ =>
                        {}
                    },

                    InputMode::Editing => match key.code
                    {
                        KeyCode::Esc | KeyCode::Enter =>
                        {
                            app.input_mode = InputMode::Normal;
                        }

                        KeyCode::Backspace =>
                        {
                            app.search_query.pop();
                            app.update_search();
                        }

                        KeyCode::Char(c) =>
                        {
                            app.search_query.push(c);
                            app.update_search();
                        }

                        _ =>
                        {}
                    },
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    terminal.show_cursor()?;

    Ok(())
}

fn ui(frame: &mut Frame, app: &mut App)
{
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1), Constraint::Length(1)])
        .split(frame.area());

    render_search(frame, app, chunks[0]);
    render_content(frame, app, chunks[1]);
    render_help(frame, app, chunks[2]);
}

fn render_search(frame: &mut Frame, app: &App, area: Rect)
{
    let search_style = match app.input_mode
    {
        InputMode::Editing => Style::default().fg(Color::Yellow),
        InputMode::Normal => Style::default().fg(Color::White),
    };

    let search_title = format!(
        " Rule-O-Matic v{} | {} ",
        env!("CARGO_PKG_VERSION"),
        if app.input_mode == InputMode::Editing
        {
            t!("search_typing")
        }
        else
        {
            t!("search_placeholder")
        }
    );

    let search_text = Paragraph::new(app.search_query.as_str()).style(search_style).block(
        Block::default()
            .borders(Borders::ALL)
            .title(search_title)
            .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
    );

    frame.render_widget(search_text, area);
}

fn render_content(frame: &mut Frame, app: &mut App, area: Rect)
{
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    render_list(frame, app, chunks[0]);
    render_details(frame, app, chunks[1]);
}

fn render_list(frame: &mut Frame, app: &mut App, area: Rect)
{
    let total_items = app.filtered_rules.len();
    let total_pages = if total_items > 0
    {
        (total_items as f64 / app.items_per_page as f64).ceil() as usize
    }
    else
    {
        1
    };

    let start_index = app.current_page * app.items_per_page;
    let end_index = (start_index + app.items_per_page).min(total_items);

    let page_items_data = if total_items > 0 && start_index < total_items
    {
        &app.filtered_rules[start_index..end_index]
    }
    else
    {
        &[]
    };

    let items: Vec<ListItem> = page_items_data
        .iter()
        .map(|rule| {
            let category = rule
                .source_file
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("root");

            let name = rule
                .data
                .name
                .as_deref()
                .map(Cow::Borrowed)
                .unwrap_or_else(|| t!("unknown"));

            let rule_type = rule.data.rule_type.as_deref().unwrap_or("-");

            let content = Line::from(vec![
                Span::styled(format!("[{}] ", category), Style::default().fg(Color::Blue)),
                Span::styled(name, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::styled(format!(" ({}) ", rule_type), Style::default().fg(Color::White)),
            ]);

            ListItem::new(content)
        })
        .collect();

    let list_title = format!(
        " {} ",
        t!("rules_page", current = app.current_page + 1, total = total_pages)
    );

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(list_title))
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");

    frame.render_stateful_widget(list, area, &mut app.list_state);
}

fn render_details(frame: &mut Frame, app: &App, area: Rect)
{
    let selected_visual_index = app.list_state.selected();
    let start_index = app.current_page * app.items_per_page;

    let detail_text = if let Some(visual_idx) = selected_visual_index
    {
        let real_index = start_index + visual_idx;

        if let Some(rule) = app.filtered_rules.get(real_index)
        {
            let mut lines = vec![Line::from(vec![
                Span::raw("Name: "),
                Span::styled(
                    rule.data.name.as_deref().unwrap_or("?"),
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                ),
            ])];

            if let Some(rule_type) = &rule.data.rule_type
            {
                lines.push(Line::from(vec![
                    Span::raw("Type: "),
                    Span::styled(rule_type, Style::default().fg(Color::White)),
                ]));
            }

            if let Some(nice) = rule.data.nice
            {
                lines.push(Line::from(vec![
                    Span::raw("Nice: "),
                    Span::styled(nice.to_string(), Style::default().fg(Color::Yellow)),
                ]));
            }

            if let Some(nice) = rule.data.latency_nice
            {
                lines.push(Line::from(vec![
                    Span::raw("Nice latency: "),
                    Span::styled(nice.to_string(), Style::default()),
                ]));
            }

            if let Some(sched) = &rule.data.sched
            {
                lines.push(Line::from(vec![
                    Span::raw("Scheduling policy: "),
                    Span::styled(sched, Style::default()),
                ]));
            }

            if let Some(rtprio) = rule.data.rtprio
            {
                lines.push(Line::from(vec![
                    Span::raw("Static priority: "),
                    Span::styled(rtprio.to_string(), Style::default()),
                ]));
            }

            if let Some(ioclass) = &rule.data.ioclass
            {
                lines.push(Line::from(vec![
                    Span::raw("IO class: "),
                    Span::styled(ioclass, Style::default()),
                ]));
            }

            if let Some(oom_score_adj) = rule.data.oom_score_adj
            {
                lines.push(Line::from(vec![
                    Span::raw("Out of memory killer score: "),
                    Span::styled(oom_score_adj.to_string(), Style::default()),
                ]));
            }

            if let Some(cgroup) = &rule.data.cgroup
            {
                lines.push(Line::from(vec![
                    Span::raw("CGroup: "),
                    Span::styled(cgroup, Style::default()),
                ]));
            }

            lines.push(Line::from(""));

            lines.push(Line::from(Span::styled(
                t!("source_file"),
                Style::default().add_modifier(Modifier::UNDERLINED),
            )));

            lines.push(Line::from(rule.source_file.to_string_lossy().to_string()));
            lines.push(Line::from(""));

            if let Some(ctx) = &rule.context_comment
            {
                lines.push(Line::from(Span::styled(
                    t!("context_comment"),
                    Style::default().add_modifier(Modifier::UNDERLINED),
                )));
                for comment_line in ctx.lines()
                {
                    lines.push(Line::from(Span::styled(comment_line, Style::default().fg(Color::Gray))));
                }
            }
            lines
        }
        else
        {
            vec![Line::from(t!("error_selecting_rule"))]
        }
    }
    else
    {
        vec![Line::from(
            if app.filtered_rules.is_empty()
            {
                t!("no_rules_found")
            }
            else
            {
                t!("no_selection")
            },
        )]
    };

    let details_block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", t!("details_title")))
        .title_bottom(
            Line::from(format!(" {} ", t!("quote_coffee")))
                .alignment(Alignment::Right)
                .style(Style::default().fg(Color::Magenta).add_modifier(Modifier::ITALIC)),
        );

    let details = Paragraph::new(detail_text)
        .block(details_block)
        .wrap(Wrap { trim: true });

    frame.render_widget(details, area);
}

fn render_help(frame: &mut Frame, app: &App, area: Rect)
{
    let help_text = match app.input_mode
    {
        InputMode::Normal => format!(" {} ", t!("help_normal")),
        InputMode::Editing => format!(" {} ", t!("help_editing")),
    };

    let help = Paragraph::new(help_text).style(Style::default().fg(Color::DarkGray));

    frame.render_widget(help, area);
}
