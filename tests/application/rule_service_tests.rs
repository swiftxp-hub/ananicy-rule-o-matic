use ananicy_rule_o_matic::application::rule_service::RuleService;
use ananicy_rule_o_matic::infrastructure::rule_repository::RuleRepository;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

fn create_rule_file(dir: &TempDir, subpath: &str, content: &str) -> PathBuf
{
    let full_path = dir.path().join(subpath);

    if let Some(parent) = full_path.parent()
    {
        fs::create_dir_all(parent).unwrap();
    }

    fs::write(&full_path, content).unwrap();

    full_path
}

#[test]
fn test_search_rules_basic()
{
    let temp_dir = TempDir::new().unwrap();

    create_rule_file(
        &temp_dir,
        "00-default/test.rules",
        r#"
        { "name": "test-process", "type": "Game" }
    "#,
    );

    let rule_repository = RuleRepository::new(vec![temp_dir.path().to_path_buf()]);
    let rule_service = RuleService::new(rule_repository);

    let rules = rule_service.search_rules("").unwrap();

    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].data.name.as_deref(), Some("test-process"));
    assert!(!rules[0].shadowed);
}

#[test]
fn test_shadowing_logic()
{
    let temp_dir = TempDir::new().unwrap();

    let default_dir = temp_dir.path().join("default");
    let custom_dir = temp_dir.path().join("custom");
    fs::create_dir_all(&default_dir).unwrap();
    fs::create_dir_all(&custom_dir).unwrap();

    let default_file = default_dir.join("game.rules");
    fs::write(&default_file, r#"{"name": "game", "nice": 0}"#).unwrap();

    let custom_file = custom_dir.join("game.rules");
    fs::write(&custom_file, r#"{"name": "game", "nice": -5}"#).unwrap();

    let rule_repository = RuleRepository::new(vec![default_dir, custom_dir]);
    let rule_service = RuleService::new(rule_repository);

    let rules = rule_service.search_rules("").unwrap();

    assert_eq!(rules.len(), 2);

    let shadowed_rule = rules.iter().find(|r| r.shadowed).expect("Should have a shadowed rule");
    let active_rule = rules.iter().find(|r| !r.shadowed).expect("Should have an active rule");

    assert_eq!(shadowed_rule.data.nice, Some(0));
    assert_eq!(active_rule.data.nice, Some(-5));
}

#[test]
fn test_filtering()
{
    let temp_dir = TempDir::new().unwrap();
    create_rule_file(
        &temp_dir,
        "test.rules",
        r#"
        {"name": "foo", "type": "Game"}
        {"name": "bar", "type": "BG"}
        {"name": "baz", "cgroup": "system.slice"}
    "#,
    );

    let rule_repository = RuleRepository::new(vec![temp_dir.path().to_path_buf()]);
    let rule_service = RuleService::new(rule_repository);

    let result = rule_service.search_rules("foo").unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].data.name.as_deref(), Some("foo"));

    let result = rule_service.search_rules("bg").unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].data.name.as_deref(), Some("bar"));

    let result = rule_service.search_rules("system").unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].data.name.as_deref(), Some("baz"));

    let result = rule_service.search_rules("nonexistent").unwrap();
    assert_eq!(result.len(), 0);
}

#[test]
fn test_sorting_by_category_and_name()
{
    let temp_dir = TempDir::new().unwrap();

    create_rule_file(&temp_dir, "a_cat/z.rules", r#"{"name": "z_rule"}"#);
    create_rule_file(&temp_dir, "b_cat/a.rules", r#"{"name": "a_rule"}"#);

    let rule_repository = RuleRepository::new(vec![temp_dir.path().to_path_buf()]);
    let rule_service = RuleService::new(rule_repository);

    let rules = rule_service.search_rules("").unwrap();
    assert_eq!(rules.len(), 2);

    assert_eq!(rules[0].data.name.as_deref(), Some("z_rule"));
    assert_eq!(rules[1].data.name.as_deref(), Some("a_rule"));
}

#[test]
fn test_sorting_same_category_by_name()
{
    let temp_dir = TempDir::new().unwrap();
    create_rule_file(
        &temp_dir,
        "cat/rules.rules",
        r#"
        {"name": "b_rule"}
        {"name": "a_rule"}
    "#,
    );

    let rule_repository = RuleRepository::new(vec![temp_dir.path().to_path_buf()]);
    let rule_service = RuleService::new(rule_repository);

    let rules = rule_service.search_rules("").unwrap();

    assert_eq!(rules[0].data.name.as_deref(), Some("a_rule"));
    assert_eq!(rules[1].data.name.as_deref(), Some("b_rule"));
}

#[test]
fn test_context_comments()
{
    let temp_dir = TempDir::new().unwrap();
    create_rule_file(
        &temp_dir,
        "test.rules",
        r#"
        # This is a comment
        {"name": "commented"}
     "#,
    );

    let rule_repository = RuleRepository::new(vec![temp_dir.path().to_path_buf()]);
    let rule_service = RuleService::new(rule_repository);

    let rules = rule_service.search_rules("").unwrap();

    assert_eq!(rules[0].context_comment.as_deref(), Some("# This is a comment"));
}

#[test]
fn test_invalid_json_is_skipped()
{
    let temp_dir = TempDir::new().unwrap();
    create_rule_file(
        &temp_dir,
        "test.rules",
        r#"
        {"name": "valid"}
        { invalid json }
        {"name": "also_valid"}
     "#,
    );

    let rule_repository = RuleRepository::new(vec![temp_dir.path().to_path_buf()]);
    let rule_service = RuleService::new(rule_repository);

    let rules = rule_service.search_rules("").unwrap();

    assert_eq!(rules.len(), 2);
    assert_eq!(rules[0].data.name.as_deref(), Some("also_valid"));
    assert_eq!(rules[1].data.name.as_deref(), Some("valid"));
}
