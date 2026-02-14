use crate::application::process_service::ProcessService;
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
use std::{
    borrow::Cow,
    io,
    time::{Duration, Instant},
};

struct App
{
    all_rules: Vec<EnrichedRule>,
    current_page: usize,
    filtered_rules: Vec<EnrichedRule>,
    input_mode: InputMode,
    items_per_page: usize,
    list_state: ListState,
    search_query: String,
}

#[derive(PartialEq)]
enum InputMode
{
    Editing,
    Normal,
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

    fn next_page(&mut self)
    {
        let total_pages = (self.filtered_rules.len() as f64 / self.items_per_page as f64).ceil() as usize;
        if total_pages > 0 && self.current_page < total_pages - 1
        {
            self.current_page += 1;
            self.list_state.select(Some(0));
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

    fn previous_page(&mut self)
    {
        if self.current_page > 0
        {
            self.current_page -= 1;
            self.list_state.select(Some(0));
        }
    }

    fn update_search(&mut self)
    {
        let query = self.search_query.to_lowercase();

        self.filtered_rules = self
            .all_rules
            .iter()
            .filter(|rule| {
                let match_str = |opt: &Option<String>| opt.as_deref().unwrap_or("").to_lowercase().contains(&query);

                let match_num = |opt: &Option<i32>| opt.map(|n| n.to_string()).unwrap_or_default().contains(&query);

                match_str(&rule.data.name)
                    || match_str(&rule.data.rule_type)
                    || match_str(&rule.data.sched)
                    || match_str(&rule.data.ioclass)
                    || match_str(&rule.data.cgroup)
                    || match_str(&rule.context_comment)
                    || match_num(&rule.data.nice)
                    || match_num(&rule.data.latency_nice)
                    || match_num(&rule.data.rtprio)
                    || match_num(&rule.data.oom_score_adj)
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
}

pub fn run_app(rule_service: &RuleService, process_service: &mut ProcessService) -> Result<()>
{
    let rules = rule_service.search_rules("")?;

    enable_raw_mode()?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(rules);

    let tick_rate = Duration::from_secs(1);
    let mut last_tick = Instant::now();

    loop
    {
        terminal.draw(|frame| ui(frame, &mut app, process_service))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if event::poll(timeout)?
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

        if last_tick.elapsed() >= tick_rate
        {
            process_service.update_processes();
            last_tick = Instant::now();
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

fn ui(frame: &mut Frame, app: &mut App, process_service: &ProcessService)
{
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1), Constraint::Length(1)])
        .split(frame.area());

    render_search(frame, app, chunks[0]);
    render_content(frame, app, process_service, chunks[1]);
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

fn render_content(frame: &mut Frame, app: &mut App, process_service: &ProcessService, area: Rect)
{
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    render_list(frame, app, process_service, chunks[0]);
    render_details(frame, app, process_service, chunks[1]);
}

fn render_list(frame: &mut Frame, app: &mut App, process_service: &ProcessService, area: Rect)
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

            let original_name = rule.data.name.as_deref().unwrap_or("");
            let mut name_display = rule
                .data
                .name
                .as_deref()
                .map(Cow::Borrowed)
                .unwrap_or_else(|| t!("unknown").into());

            if rule.shadowed
            {
                name_display.to_mut().push_str(" (Shadowed)");
            }

            let is_active = process_service.is_process_active(original_name);

            let name_style = if rule.shadowed
            {
                Style::default().fg(Color::DarkGray)
            }
            else if is_active
            {
                name_display.to_mut().push_str(" [ACTIVE]");
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            }
            else
            {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            };

            let rule_type = rule.data.rule_type.as_deref().unwrap_or("-");

            let content = Line::from(vec![
                Span::styled(format!("[{:^7}] ", category), Style::default().fg(Color::Blue)),
                Span::styled(name_display, name_style),
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

fn render_details(frame: &mut Frame, app: &App, process_service: &ProcessService, area: Rect)
{
    let selected_visual_index = app.list_state.selected();
    let start_index = app.current_page * app.items_per_page;

    let detail_text = if let Some(visual_idx) = selected_visual_index
    {
        let real_index = start_index + visual_idx;

        if let Some(rule) = app.filtered_rules.get(real_index)
        {
            let rule_name = rule.data.name.as_deref().unwrap_or("");
            let running_processes = process_service.get_process_infos(rule_name);
            let current_proc = running_processes.first();

            let compare_i32 = |target: Option<i32>, actual: Option<i32>| -> Span {
                match (target, actual)
                {
                    (Some(t), Some(a)) if t == a =>
                    {
                        Span::styled(format!("(Current: {})", a), Style::default().fg(Color::Green))
                    }
                    (Some(_), Some(a)) => Span::styled(format!("(Current: {})", a), Style::default().fg(Color::Red)),
                    (None, Some(a)) => Span::styled(format!("(Current: {})", a), Style::default().fg(Color::DarkGray)),
                    _ => Span::raw(""),
                }
            };

            let compare_str = |target: &Option<String>, actual: Option<String>| -> Span {
                match (target, actual)
                {
                    (Some(t), Some(ref a)) if t.eq_ignore_ascii_case(a) =>
                    {
                        Span::styled(format!("(Current: {})", a), Style::default().fg(Color::Green))
                    }
                    (Some(_), Some(ref a)) =>
                    {
                        Span::styled(format!("(Current: {})", a), Style::default().fg(Color::Red))
                    }
                    (None, Some(ref a)) =>
                    {
                        Span::styled(format!("(Current: {})", a), Style::default().fg(Color::DarkGray))
                    }
                    _ => Span::raw(""),
                }
            };

            let mut lines = Vec::new();

            if rule.shadowed
            {
                lines.push(Line::from(Span::styled(
                    "Warning: This rule is shadowed by another rule!",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                )));
            }

            if !running_processes.is_empty()
            {
                lines.push(Line::from(Span::styled(
                    format!(
                        "Status: Running (PID: {}, Process: {})",
                        running_processes[0].pid, running_processes[0].name
                    ),
                    Style::default().fg(Color::Green),
                )));
            }
            else
            {
                lines.push(Line::from(Span::styled(
                    "Status: Not running",
                    Style::default().fg(Color::DarkGray),
                )));
            }

            lines.push(Line::from(vec![
                Span::raw("Name: "),
                Span::styled(rule_name, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            ]));

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
                    compare_i32(Some(nice), current_proc.and_then(|p| p.nice)),
                ]));
            }
            else if let Some(p) = current_proc
            {
                lines.push(Line::from(vec![Span::raw("Nice: - "), compare_i32(None, p.nice)]));
            }

            if let Some(lat) = rule.data.latency_nice
            {
                lines.push(Line::from(vec![
                    Span::raw("Nice latency: "),
                    Span::styled(lat.to_string(), Style::default()),
                    compare_i32(Some(lat), current_proc.and_then(|p| p.latency_nice)),
                ]));
            }
            else if let Some(p) = current_proc
            {
                if let Some(val) = p.latency_nice
                {
                    lines.push(Line::from(vec![
                        Span::raw("Nice latency: - "),
                        compare_i32(None, Some(val)),
                    ]));
                }
            }

            if let Some(sched) = &rule.data.sched
            {
                lines.push(Line::from(vec![
                    Span::raw("Scheduling policy: "),
                    Span::styled(sched, Style::default()),
                    compare_str(&Some(sched.clone()), current_proc.and_then(|p| p.sched_policy.clone())),
                ]));
            }
            else if let Some(p) = current_proc
            {
                lines.push(Line::from(vec![
                    Span::raw("Scheduling policy: - "),
                    compare_str(&None, p.sched_policy.clone()),
                ]));
            }

            if let Some(rtprio) = rule.data.rtprio
            {
                lines.push(Line::from(vec![
                    Span::raw("Static priority: "),
                    Span::styled(rtprio.to_string(), Style::default()),
                    compare_i32(Some(rtprio), current_proc.and_then(|p| p.rtprio)),
                ]));
            }
            else if let Some(p) = current_proc
            {
                if let Some(val) = p.rtprio
                {
                    lines.push(Line::from(vec![
                        Span::raw("Static priority: - "),
                        compare_i32(None, Some(val)),
                    ]));
                }
            }

            if let Some(ioclass) = &rule.data.ioclass
            {
                lines.push(Line::from(vec![
                    Span::raw("IO class: "),
                    Span::styled(ioclass, Style::default()),
                    compare_str(&Some(ioclass.clone()), current_proc.and_then(|p| p.ioclass.clone())),
                ]));
            }
            else if let Some(p) = current_proc
            {
                lines.push(Line::from(vec![
                    Span::raw("IO class: - "),
                    compare_str(&None, p.ioclass.clone()),
                ]));
            }

            if let Some(oom_score_adj) = rule.data.oom_score_adj
            {
                lines.push(Line::from(vec![
                    Span::raw("Out of memory killer score: "),
                    Span::styled(oom_score_adj.to_string(), Style::default()),
                    compare_i32(Some(oom_score_adj), current_proc.and_then(|p| p.oom_score_adj)),
                ]));
            }
            else if let Some(p) = current_proc
            {
                lines.push(Line::from(vec![
                    Span::raw("Out of memory killer score: - "),
                    compare_i32(None, p.oom_score_adj),
                ]));
            }

            if let Some(cgroup) = &rule.data.cgroup
            {
                let current_cgroup = current_proc.and_then(|p| p.cgroup.clone());
                let style = if current_cgroup
                    .as_ref()
                    .map(|c| c.eq_ignore_ascii_case(cgroup))
                    .unwrap_or(false)
                {
                    Style::default().fg(Color::Green)
                }
                else if current_cgroup.is_some()
                {
                    Style::default().fg(Color::Red)
                }
                else
                {
                    Style::default().fg(Color::DarkGray)
                };

                let current_display = current_cgroup
                    .map(|c| format!("(Current: {})", shorten_cgroup(&c)))
                    .unwrap_or_default();

                lines.push(Line::from(vec![
                    Span::raw("Cgroup: "),
                    Span::styled(shorten_cgroup(cgroup), Style::default()),
                    Span::styled(current_display, style),
                ]));
            }
            else if let Some(p) = current_proc
            {
                if let Some(cgroup) = &p.cgroup
                {
                    lines.push(Line::from(vec![
                        Span::raw("Cgroup: - "),
                        Span::styled(
                            format!("(Current: {})", shorten_cgroup(cgroup)),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ]));
                }
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

    let inner_height = area.height.saturating_sub(2);
    let inner_width = area.width.saturating_sub(2) as usize;

    let mut required_lines = 0;
    for line in &detail_text
    {
        let line_width = line.width();
        if line_width == 0
        {
            required_lines += 1;
        }
        else
        {
            required_lines += (line_width + inner_width - 1) / inner_width;
        }
    }

    let has_overflow = required_lines > inner_height as usize;

    let bottom_title = if has_overflow
    {
        Line::from(" (▼ More...) ").alignment(Alignment::Right).style(
            Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD | Modifier::SLOW_BLINK),
        )
    }
    else
    {
        Line::from(format!(" {} ", t!("quote_coffee")))
            .alignment(Alignment::Right)
            .style(Style::default().fg(Color::Magenta).add_modifier(Modifier::ITALIC))
    };

    let details_block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", t!("details_title")))
        .title_bottom(bottom_title);

    let details = Paragraph::new(detail_text)
        .block(details_block)
        .wrap(Wrap { trim: true });

    frame.render_widget(details, area);
}

fn render_help(frame: &mut Frame, app: &App, area: Rect)
{
    let help_text = match app.input_mode
    {
        InputMode::Editing => format!(" {} ", t!("help_editing")),
        InputMode::Normal => format!(" {} ", t!("help_normal")),
    };

    let help = Paragraph::new(help_text).style(Style::default().fg(Color::DarkGray));

    frame.render_widget(help, area);
}

fn shorten_cgroup(path: &str) -> String
{
    if path == "/"
    {
        return path.to_string();
    }

    let parts: Vec<&str> = path.split('/').collect();

    if parts.len() > 4 && path.starts_with("/user.slice")
    {
        let end = parts[parts.len().saturating_sub(2)..].join("/");
        return format!(".../{}", end);
    }

    path.to_string()
}
