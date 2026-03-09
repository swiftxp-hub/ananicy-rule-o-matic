use crate::application::process_service::ProcessService;
use crate::application::rule_service::RuleService;
use crate::domain::models::{AnanicyRule, EnrichedRule};

use anyhow::Result;

use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
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
    rules_errors: Vec<String>,
    current_page: usize,
    filter_active_only: bool,
    filtered_rules: Vec<EnrichedRule>,
    input_mode: InputMode,
    items_per_page: usize,
    list_state: ListState,
    search_query: String,

    // Editing fields
    editing_rule: AnanicyRule,
    editing_field_index: usize,
    editing_buffer: String,
    notification: Option<(String, Color)>,
    notification_time: Option<Instant>,

    // Process Selection
    process_list: Vec<String>,
    process_list_state: ListState,

    // Permissions
    is_root: bool,
}

#[derive(PartialEq)]
enum InputMode
{
    Editing, // Searching
    Normal,
    RuleForm, // Creating/Editing Rule
}

impl App
{
    fn new(rules: Vec<EnrichedRule>, errors: Vec<String>) -> Self
    {
        let is_root = unsafe { libc::geteuid() == 0 };

        let mut app = Self {
            all_rules: rules.clone(),
            rules_errors: errors,
            filtered_rules: rules,
            filter_active_only: false,
            list_state: ListState::default(),
            search_query: String::new(),
            input_mode: InputMode::Normal,
            current_page: 0,
            items_per_page: 50,
            editing_rule: AnanicyRule::default(),
            editing_field_index: 0,
            editing_buffer: String::new(),
            notification: None,
            notification_time: None,
            process_list: Vec::new(),
            process_list_state: ListState::default(),
            is_root,
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

    fn update_search(&mut self, process_service: &ProcessService)
    {
        let selected_rule_name = self
            .list_state
            .selected()
            .and_then(|idx| {
                let real_idx = self.current_page * self.items_per_page + idx;
                self.filtered_rules.get(real_idx)
            })
            .and_then(|rule| rule.data.name.clone());

        let query = self.search_query.to_lowercase();

        self.filtered_rules = self
            .all_rules
            .iter()
            .filter(|rule| {
                if self.filter_active_only
                {
                    let original_name = rule.data.name.as_deref().unwrap_or("");
                    if !process_service.is_process_active(original_name)
                    {
                        return false;
                    }
                }

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

        if let Some(name) = selected_rule_name
        {
            if let Some(new_idx) = self
                .filtered_rules
                .iter()
                .position(|r| r.data.name == Some(name.clone()))
            {
                self.current_page = new_idx / self.items_per_page;
                let visual_idx = new_idx % self.items_per_page;
                self.list_state.select(Some(visual_idx));
                return;
            }
        }

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

    fn get_field_value(&self, index: usize) -> String
    {
        match index
        {
            0 => self.editing_rule.name.clone().unwrap_or_default(),
            1 => self.editing_rule.rule_type.clone().unwrap_or_default(),
            2 => self.editing_rule.nice.map(|v| v.to_string()).unwrap_or_default(),
            3 => self
                .editing_rule
                .latency_nice
                .map(|v| v.to_string())
                .unwrap_or_default(),
            4 => self.editing_rule.sched.clone().unwrap_or_default(),
            5 => self.editing_rule.rtprio.map(|v| v.to_string()).unwrap_or_default(),
            6 => self.editing_rule.ioclass.clone().unwrap_or_default(),
            7 => self
                .editing_rule
                .oom_score_adj
                .map(|v| v.to_string())
                .unwrap_or_default(),
            8 => self.editing_rule.cgroup.clone().unwrap_or_default(),
            _ => String::new(),
        }
    }

    fn set_field_value(&mut self, index: usize, value: String)
    {
        let value = if value.trim().is_empty() { None } else { Some(value) };

        match index
        {
            0 => self.editing_rule.name = value,
            1 => self.editing_rule.rule_type = value,
            2 => self.editing_rule.nice = value.as_deref().and_then(|v| v.parse().ok()),
            3 => self.editing_rule.latency_nice = value.as_deref().and_then(|v| v.parse().ok()),
            4 => self.editing_rule.sched = value,
            5 => self.editing_rule.rtprio = value.as_deref().and_then(|v| v.parse().ok()),
            6 => self.editing_rule.ioclass = value,
            7 => self.editing_rule.oom_score_adj = value.as_deref().and_then(|v| v.parse().ok()),
            8 => self.editing_rule.cgroup = value,
            _ =>
            {}
        }
    }

    fn start_editing(&mut self, rule: Option<AnanicyRule>)
    {
        self.editing_rule = rule.unwrap_or_default();
        self.editing_field_index = 0;
        self.editing_buffer = self.get_field_value(0);
        self.input_mode = InputMode::RuleForm;
        self.process_list.clear();
        self.process_list_state.select(None);
    }

    fn save_field_buffer(&mut self)
    {
        self.set_field_value(self.editing_field_index, self.editing_buffer.clone());
    }

    fn move_edit_field(&mut self, delta: i32)
    {
        self.save_field_buffer();
        let new_index = (self.editing_field_index as i32 + delta).rem_euclid(9) as usize;
        self.editing_field_index = new_index;
        self.editing_buffer = self.get_field_value(new_index);
        // Do not clear process list, keep it for reference
    }

    fn update_process_search(&mut self, process_service: &ProcessService)
    {
        if self.input_mode == InputMode::RuleForm
        {
            // Always update suggestions based on name field (index 0)
            let query = if self.editing_field_index == 0
            {
                self.editing_buffer.clone()
            }
            else
            {
                self.get_field_value(0)
            };

            self.process_list = process_service.search_processes(&query);
            if !self.process_list.is_empty()
            {
                // Only auto-select if we are actively editing the name field
                if self.editing_field_index == 0
                {
                    self.process_list_state.select(Some(0));
                }
            }
            else
            {
                self.process_list_state.select(None);
            }
        }
    }
}

pub fn run_app(rule_service: &RuleService, process_service: &mut ProcessService) -> Result<()>
{
    let (rules, errors) = rule_service.search_rules("")?;

    enable_raw_mode()?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(rules, errors);

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
                        KeyCode::Char('n') =>
                        {
                            if app.is_root
                            {
                                app.start_editing(None);
                                app.update_process_search(process_service);
                            }
                            else
                            {
                                app.notification = Some(("Root required to create new rules.".to_string(), Color::Red));
                                app.notification_time = Some(Instant::now());
                            }
                        }
                        KeyCode::Char('e') =>
                        {
                            if app.is_root
                            {
                                if let Some(selected) = app.list_state.selected()
                                {
                                    let start = app.current_page * app.items_per_page;
                                    let real_idx = start + selected;
                                    if let Some(rule) = app.filtered_rules.get(real_idx)
                                    {
                                        app.start_editing(Some(rule.data.clone()));
                                        app.update_process_search(process_service);
                                    }
                                }
                            }
                            else
                            {
                                app.notification = Some(("Root required to edit rules.".to_string(), Color::Red));
                                app.notification_time = Some(Instant::now());
                            }
                        }
                        KeyCode::Char('a') =>
                        {
                            app.filter_active_only = !app.filter_active_only;
                            app.update_search(process_service);
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
                            app.update_search(process_service);
                        }
                        KeyCode::Char(c) =>
                        {
                            app.search_query.push(c);
                            app.update_search(process_service);
                        }
                        _ =>
                        {}
                    },
                    InputMode::RuleForm => match key.code
                    {
                        KeyCode::Esc =>
                        {
                            app.input_mode = InputMode::Normal;
                        }
                        KeyCode::Tab =>
                        {
                            app.move_edit_field(1);
                        }
                        KeyCode::BackTab =>
                        {
                            app.move_edit_field(-1);
                        }
                        KeyCode::Down =>
                        {
                            if app.editing_field_index == 0 && !app.process_list.is_empty()
                            {
                                let current = app.process_list_state.selected().unwrap_or(0);
                                let next = (current + 1).min(app.process_list.len() - 1);
                                app.process_list_state.select(Some(next));
                            }
                            else
                            {
                                app.move_edit_field(1);
                            }
                        }
                        KeyCode::Up =>
                        {
                            if app.editing_field_index == 0 && !app.process_list.is_empty()
                            {
                                let current = app.process_list_state.selected().unwrap_or(0);
                                if current > 0
                                {
                                    app.process_list_state.select(Some(current - 1));
                                }
                                else
                                {
                                    app.move_edit_field(-1);
                                }
                            }
                            else
                            {
                                app.move_edit_field(-1);
                            }
                        }
                        KeyCode::Enter =>
                        {
                            if app.editing_field_index == 0 && !app.process_list.is_empty()
                            {
                                if let Some(idx) = app.process_list_state.selected()
                                {
                                    if let Some(selected_process) = app.process_list.get(idx)
                                    {
                                        let first_part =
                                            selected_process.split_whitespace().next().unwrap_or(selected_process);
                                        let name = std::path::Path::new(first_part)
                                            .file_name()
                                            .and_then(|n| n.to_str())
                                            .unwrap_or(first_part);

                                        app.editing_buffer = name.to_string();
                                        // Keep list visible but filtered
                                        // app.process_list.clear();
                                    }
                                }
                                app.move_edit_field(1);
                            }
                            else
                            {
                                app.move_edit_field(1);
                            }
                        }
                        KeyCode::Backspace =>
                        {
                            app.editing_buffer.pop();
                            if app.editing_field_index == 0
                            {
                                app.update_process_search(process_service);
                            }
                        }
                        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) =>
                        {
                            app.save_field_buffer();
                            match rule_service.save_rule(&app.editing_rule)
                            {
                                Ok(_) =>
                                {
                                    app.notification = Some(("Rule saved successfully!".to_string(), Color::Green));
                                    app.input_mode = InputMode::Normal;
                                    // Reload rules
                                    if let Ok((rules, errors)) = rule_service.search_rules("")
                                    {
                                        app.all_rules = rules;
                                        app.rules_errors.extend(errors);
                                        app.update_search(process_service);
                                    }
                                }
                                Err(e) =>
                                {
                                    app.notification = Some((format!("Error saving: {}", e), Color::Red));
                                }
                            }
                            app.notification_time = Some(Instant::now());
                        }
                        KeyCode::Char(c) =>
                        {
                            app.editing_buffer.push(c);
                            if app.editing_field_index == 0
                            {
                                app.update_process_search(process_service);
                            }
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

            if app.filter_active_only
            {
                app.update_search(process_service);
            }

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
    let layout_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1), Constraint::Length(1)])
        .split(frame.area());

    render_search(frame, app, layout_chunks[0]);
    render_content(frame, app, process_service, layout_chunks[1]);

    if let Some((msg, color)) = &app.notification
    {
        if let Some(time) = app.notification_time
        {
            if time.elapsed() < Duration::from_secs(3)
            {
                let notif_area = Rect {
                    x: layout_chunks[0].x + layout_chunks[0].width / 2
                        - (msg.len() as u16 / 2).min(layout_chunks[0].width / 2),
                    y: layout_chunks[0].y + 1,
                    width: (msg.len() as u16 + 4).min(layout_chunks[0].width),
                    height: 1,
                };
                let notif = Paragraph::new(msg.as_str()).style(
                    Style::default()
                        .bg(*color)
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                );
                frame.render_widget(notif, notif_area);
            }
        }
    }

    render_help(frame, app, layout_chunks[2]);
}

fn render_search(frame: &mut Frame, app: &App, area: Rect)
{
    let search_style = match app.input_mode
    {
        InputMode::Editing => Style::default().fg(Color::Yellow),
        InputMode::Normal => Style::default().fg(Color::White),
        InputMode::RuleForm => Style::default().fg(Color::DarkGray),
    };

    let mut search_title = format!(
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

    if app.filter_active_only
    {
        search_title.push_str(&t!("active_filter_enabled"));
    }

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
    if app.input_mode == InputMode::RuleForm
    {
        let v_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(13), Constraint::Min(1)])
            .split(area);

        let h_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(v_chunks[0]);

        render_rule_form(frame, app, h_chunks[0]);
        render_details(frame, app, process_service, h_chunks[1]);
        render_process_list(frame, app, v_chunks[1]);
    }
    else
    {
        let layout_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        render_list(frame, app, process_service, layout_chunks[0]);
        render_details(frame, app, process_service, layout_chunks[1]);
    }
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

    let mut list_block = Block::default().borders(Borders::ALL).title(list_title);

    if !app.rules_errors.is_empty()
    {
        let error_msg = format!(" {} ", t!("error_loading_files", count = app.rules_errors.len()));

        list_block = list_block.title_bottom(
            Line::from(error_msg)
                .style(Style::default().fg(Color::Red))
                .alignment(Alignment::Left),
        );
    }

    let list = List::new(items)
        .block(list_block)
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");

    frame.render_stateful_widget(list, area, &mut app.list_state);
}

fn render_details(frame: &mut Frame, app: &App, process_service: &ProcessService, area: Rect)
{
    let (target_rule, source_file, context_comment, shadowed) = if app.input_mode == InputMode::RuleForm
    {
        (&app.editing_rule, None, None, false)
    }
    else if let Some(visual_idx) = app.list_state.selected()
    {
        let start_index = app.current_page * app.items_per_page;
        let real_index = start_index + visual_idx;
        if let Some(rule) = app.filtered_rules.get(real_index)
        {
            (
                &rule.data,
                Some(rule.source_file.to_string_lossy()),
                rule.context_comment.as_deref(),
                rule.shadowed,
            )
        }
        else
        {
            // Fallback if selection is invalid
            let no_sel = vec![Line::from(t!("error_selecting_rule"))];
            let details = Paragraph::new(no_sel).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" {} ", t!("details_title"))),
            );
            frame.render_widget(details, area);
            return;
        }
    }
    else
    {
        let msg = if app.filtered_rules.is_empty()
        {
            t!("no_rules_found")
        }
        else
        {
            t!("no_selection")
        };
        let details = Paragraph::new(vec![Line::from(msg)]).block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" {} ", t!("details_title"))),
        );
        frame.render_widget(details, area);
        return;
    };

    let rule_name = target_rule.name.as_deref().unwrap_or("");
    let running_processes = process_service.get_process_infos(rule_name);
    let current_proc = running_processes.first();

    let compare_i32 = |target: Option<i32>, actual: Option<i32>| -> Span {
        match (target, actual)
        {
            (Some(t), Some(a)) if t == a =>
            {
                Span::styled(format!(" (Current: {})", a), Style::default().fg(Color::Green))
            }
            (Some(_), Some(a)) => Span::styled(format!(" (Current: {})", a), Style::default().fg(Color::Red)),
            (None, Some(a)) => Span::styled(format!(" (Current: {})", a), Style::default().fg(Color::DarkGray)),
            _ => Span::raw(""),
        }
    };

    let compare_str = |target: &Option<String>, actual: Option<String>| -> Span {
        match (target, actual)
        {
            (Some(t), Some(ref a)) if t.eq_ignore_ascii_case(a) =>
            {
                Span::styled(format!(" (Current: {})", a), Style::default().fg(Color::Green))
            }
            (Some(_), Some(ref a)) => Span::styled(format!(" (Current: {})", a), Style::default().fg(Color::Red)),
            (None, Some(ref a)) => Span::styled(format!(" (Current: {})", a), Style::default().fg(Color::DarkGray)),
            _ => Span::raw(""),
        }
    };

    let mut lines = Vec::new();

    if app.input_mode == InputMode::RuleForm
    {
        lines.push(Line::from(Span::styled(
            " --- PREVIEW --- ",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )));
    }

    if shadowed
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
                running_processes[0].process_id, running_processes[0].name
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

    if let Some(rule_type) = &target_rule.rule_type
    {
        lines.push(Line::from(vec![
            Span::raw("Type: "),
            Span::styled(rule_type, Style::default().fg(Color::White)),
        ]));
    }

    if let Some(nice) = target_rule.nice
    {
        lines.push(Line::from(vec![
            Span::raw("Nice: "),
            Span::styled(nice.to_string(), Style::default().fg(Color::Yellow)),
            compare_i32(Some(nice), current_proc.and_then(|p| p.nice)),
        ]));
    }
    else if let Some(p) = current_proc
    {
        lines.push(Line::from(vec![Span::raw("Nice: -"), compare_i32(None, p.nice)]));
    }

    if let Some(lat) = target_rule.latency_nice
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
                Span::raw("Nice latency: -"),
                compare_i32(None, Some(val)),
            ]));
        }
    }

    if let Some(sched) = &target_rule.sched
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
            Span::raw("Scheduling policy: -"),
            compare_str(&None, p.sched_policy.clone()),
        ]));
    }

    if let Some(rtprio) = target_rule.rtprio
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
                Span::raw("Static priority: -"),
                compare_i32(None, Some(val)),
            ]));
        }
    }

    if let Some(ioclass) = &target_rule.ioclass
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
            Span::raw("IO class: -"),
            compare_str(&None, p.ioclass.clone()),
        ]));
    }

    if let Some(oom_score_adj) = target_rule.oom_score_adj
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
            Span::raw("Out of memory killer score: -"),
            compare_i32(None, p.oom_score_adj),
        ]));
    }

    if let Some(cgroup) = &target_rule.cgroup
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
            .map(|c| format!(" (Current: {})", ProcessService::shorten_cgroup(&c)))
            .unwrap_or_default();

        lines.push(Line::from(vec![
            Span::raw("Cgroup: "),
            Span::styled(ProcessService::shorten_cgroup(cgroup), Style::default()),
            Span::styled(current_display, style),
        ]));
    }
    else if let Some(p) = current_proc
    {
        if let Some(cgroup) = &p.cgroup
        {
            lines.push(Line::from(vec![
                Span::raw("Cgroup: -"),
                Span::styled(
                    format!(" (Current: {})", ProcessService::shorten_cgroup(cgroup)),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        }
    }

    if let Some(src) = source_file
    {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            t!("source_file"),
            Style::default().add_modifier(Modifier::UNDERLINED),
        )));
        lines.push(Line::from(src.to_string()));
    }

    if let Some(ctx) = context_comment
    {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            t!("context_comment"),
            Style::default().add_modifier(Modifier::UNDERLINED),
        )));
        for comment_line in ctx.lines()
        {
            lines.push(Line::from(Span::styled(comment_line, Style::default().fg(Color::Gray))));
        }
    }

    let inner_height = area.height.saturating_sub(2);
    let inner_width = area.width.saturating_sub(2) as usize;

    let mut required_lines = 0;

    for line in &lines
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

    let details = Paragraph::new(lines).block(details_block).wrap(Wrap { trim: true });

    frame.render_widget(details, area);
}

fn render_help(frame: &mut Frame, app: &App, area: Rect)
{
    let help_text = match app.input_mode
    {
        InputMode::Editing => Line::from(format!(" {} ", t!("help_editing"))),
        InputMode::Normal =>
        {
            let base = format!(" {} ", t!("help_normal"));
            if app.is_root
            {
                Line::from(format!("{}| n: New Rule | e: Edit Rule ", base))
            }
            else
            {
                Line::from(vec![
                    Span::raw(base),
                    Span::raw("| "),
                    Span::styled("Read-only mode ", Style::default().fg(Color::Yellow)),
                ])
            }
        }
        InputMode::RuleForm => Line::from(" Esc: Cancel | Tab: Next | Enter: Select/Next | Ctrl+S: Save "),
    };

    let help = Paragraph::new(help_text).style(Style::default().fg(Color::DarkGray));

    frame.render_widget(help, area);
}

fn render_rule_form(frame: &mut Frame, app: &mut App, area: Rect)
{
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Edit Rule ")
        .title_style(Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD))
        .border_style(Style::default().fg(Color::Yellow));

    frame.render_widget(block.clone(), area);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(0),
            ]
            .as_ref(),
        )
        .split(area);

    let fields = [
        "Name",
        "Type",
        "Nice",
        "Latency Nice",
        "Sched",
        "Rtprio",
        "IO Class",
        "OOM Score Adj",
        "Cgroup",
    ];

    for (i, field_name) in fields.iter().enumerate()
    {
        let is_selected = i == app.editing_field_index;

        let value = if is_selected
        {
            app.editing_buffer.clone()
        }
        else
        {
            app.get_field_value(i)
        };

        let (prefix, bg_style, label_fg) = if is_selected
        {
            (">> ", Style::default().bg(Color::DarkGray), Color::Yellow)
        }
        else
        {
            ("   ", Style::default(), Color::Cyan)
        };

        let value_style = if is_selected
        {
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
        }
        else
        {
            Style::default().fg(Color::White)
        };

        let line = Line::from(vec![
            Span::styled(prefix, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled(
                format!("{:15}: ", field_name),
                Style::default()
                    .fg(label_fg)
                    .add_modifier(if is_selected { Modifier::BOLD } else { Modifier::empty() }),
            ),
            Span::styled(value, value_style),
        ]);

        if i < layout.len()
        {
            frame.render_widget(Paragraph::new(line).style(bg_style), layout[i]);
        }
    }
}

fn render_process_list(frame: &mut Frame, app: &mut App, area: Rect)
{
    let items: Vec<ListItem> = app
        .process_list
        .iter()
        .map(|name| ListItem::new(Line::from(Span::raw(name))))
        .collect();

    let title = " Processes ";

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD));

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");

    frame.render_stateful_widget(list, area, &mut app.process_list_state);
}
